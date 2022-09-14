use crate::channel::{ClosedChannel, Sender};
use crate::interface::InputConnector;
use crate::message::Message;

use async_trait::async_trait;
use imap::{error::Error, Session};
use mailparse::{DispositionType, MailHeaderMap, ParsedMail};
use native_tls::{TlsConnector, TlsStream};

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

struct GmailOAuth2 {
    user: String,
    access_token: String,
}

impl imap::Authenticator for GmailOAuth2 {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

/// Input connector that acts as an IMAP client
/// The service fetchs and removes the email from the server, and transforms it to messages.
/// The first word of the subjet is interpreted as the service name.
/// The following spaced-separated words are the arguments.
///
/// This connector makes attempts to the ICMP server each [`ImapClient::polling_time`] seconds.
#[derive(Default, Clone)]
pub struct ImapClient {
    imap_domain: String,
    email: String,
    secret: String,
    oauth2: bool,
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

    pub fn secret(mut self, value: impl Into<String>) -> Self {
        self.secret = value.into();
        self
    }

    /// Use the secret as an access token for oauth2.
    pub fn oauth2(mut self, value: bool) -> Self {
        self.oauth2 = value;
        self
    }

    pub fn polling_time(mut self, duration: Duration) -> Self {
        self.polling_time = duration;
        self
    }

    fn connect(&self) -> Result<Session<TlsStream<TcpStream>>, Error> {
        let tls = TlsConnector::builder().build().unwrap();
        let client = imap::connect(
            (self.imap_domain.as_str(), 993),
            self.imap_domain.as_str(),
            &tls,
        )?;
        match self.oauth2 {
            false => client.login(&self.email, &self.secret).map_err(|e| e.0),
            true => {
                let gmail_auth = GmailOAuth2 {
                    user: self.email.clone(),
                    access_token: self.secret.clone(),
                };
                client.authenticate("XOAUTH2", &gmail_auth).map_err(|e| e.0)
            }
        }
    }
}

#[async_trait]
impl InputConnector for ImapClient {
    async fn run(mut self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel> {
        tokio::task::spawn_blocking(move || {
            let mut session = self.connect().unwrap();
            loop {
                std::thread::sleep(self.polling_time);

                match read_inbox(&mut session) {
                    Ok(Some(message)) => sender.blocking_send(message)?,
                    Ok(None) => (),
                    Err(err) => {
                        log::warn!("{}", err);
                        session = match self.connect() {
                            Ok(session) => {
                                log::info!("Connection restored");
                                session
                            }
                            Err(err) => {
                                log::error!("{}", err);
                                continue;
                            }
                        }
                    }
                }
            }
        })
        .await
        .unwrap()
    }
}

fn read_inbox<T: Read + Write>(session: &mut Session<T>) -> Result<Option<Message>, Error> {
    session.select("INBOX")?;

    let emails = session.fetch("1", "RFC822")?;

    if let Some(email) = emails.iter().next() {
        session.store(format!("{}", email.message), "+FLAGS (\\Deleted)")?;

        session.expunge()?;

        if let Some(body) = email.body() {
            log::trace!(
                "Raw email:\n{}",
                std::str::from_utf8(body).unwrap_or("No utf8")
            );

            match mailparse::parse_mail(body) {
                Ok(parsed) => return Ok(Some(email_to_message(parsed))),
                Err(err) => log::error!("{}", err),
            }
        }
    }

    Ok(None)
}

fn email_to_message(email: ParsedMail) -> Message {
    let subject = email.headers.get_first_value("Subject").unwrap_or_default();
    let mut subject_args = subject.split_whitespace().map(|s| s.to_owned());

    let mut body = String::default();
    for part in &email.subparts {
        if part.ctype.mimetype.starts_with("text/plain") {
            body = part.get_body().unwrap_or_default();
        }
    }

    let mut files = HashMap::default();
    for part in &email.subparts {
        let content_disposition = part.get_content_disposition();
        if let DispositionType::Attachment = content_disposition.disposition {
            if let Some(filename) = content_disposition.params.get("filename") {
                files.insert(filename.into(), part.get_body_raw().unwrap_or_default());
            }
        }
    }

    Message {
        user: email
            .headers
            .get_first_value("From")
            .map(|from_list| {
                mailparse::addrparse(&from_list)
                    .expect("Have a 'From' email")
                    .extract_single_info()
                    .expect("Have at least one 'From' email")
                    .addr
            })
            .unwrap_or_default(),
        service_name: subject_args.next().unwrap_or_default(),
        args: subject_args.collect(),
        body,
        attached_data: files,
    }
}
