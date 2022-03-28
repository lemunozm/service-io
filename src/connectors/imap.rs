use crate::interface::{InputConnector, Message};

use async_trait::async_trait;
use mailparse::{DispositionType, MailHeaderMap, ParsedMail};
use native_tls::TlsConnector;
use tokio::sync::mpsc;

use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
pub struct ImapClient {
    imap_domain: String,
    email: String,
    password: String,
    polling_time: Duration,
}

impl ImapClient {
    pub fn domain(mut self, value: impl Into<String>) -> Self {
        self.imap_domain = value.into();
        self
    }

    pub fn email(mut self, value: impl Into<String>) -> Self {
        self.email = value.into();
        self
    }

    pub fn password(mut self, value: impl Into<String>) -> Self {
        self.password = value.into();
        self
    }

    pub fn polling_time(mut self, duration: Duration) -> Self {
        self.polling_time = duration;
        self
    }
}

#[async_trait]
impl InputConnector for ImapClient {
    async fn run(mut self: Box<Self>, sender: mpsc::Sender<Message>) {
        tokio::task::spawn_blocking(move || {
            let tls = TlsConnector::builder().build().unwrap();
            let client = imap::connect(
                (self.imap_domain.as_str(), 993),
                self.imap_domain.as_str(),
                &tls,
            )
            .unwrap();
            let mut imap_session = client
                .login(self.email, self.password)
                .map_err(|e| e.0)
                .unwrap();

            loop {
                std::thread::sleep(self.polling_time);

                imap_session.select("INBOX").unwrap();

                let emails = imap_session.fetch("1", "RFC822").unwrap();

                if let Some(email) = emails.iter().next() {
                    imap_session
                        .store(format!("{}", email.message), "+FLAGS (\\Deleted)")
                        .unwrap();

                    imap_session.expunge().unwrap();

                    if let Some(body) = email.body() {
                        let parsed = mailparse::parse_mail(body).unwrap();
                        let message = email_to_message(parsed);
                        sender.blocking_send(message).unwrap();
                    }
                }
            }
        })
        .await
        .unwrap();
    }
}

fn email_to_message(email: ParsedMail) -> Message {
    let subject = email.headers.get_first_value("Subject").unwrap_or_default();
    let mut subject_args = subject.split_whitespace().map(|s| s.to_owned());

    let mut body = String::default();
    for part in &email.subparts {
        if part.ctype.mimetype.starts_with("text/plain") {
            body = part.get_body().unwrap();
        }
    }

    let mut files = HashMap::default();
    for part in &email.subparts {
        let content_disposition = part.get_content_disposition();
        if let DispositionType::Attachment = content_disposition.disposition {
            if let Some(filename) = content_disposition.params.get("filename") {
                files.insert(filename.into(), part.get_body_raw().unwrap());
            }
        }
    }

    Message {
        user: email
            .headers
            .get_first_value("From")
            .map(|from_list| {
                mailparse::addrparse(&from_list)
                    .unwrap()
                    .extract_single_info()
                    .unwrap()
                    .addr
            })
            .unwrap_or_default(),
        service: subject_args.next().unwrap_or_default(),
        args: subject_args.collect(),
        body,
        files: files,
    }
}
