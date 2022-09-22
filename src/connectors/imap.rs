use crate::channel::{ClosedChannel, Sender};
use crate::interface::InputConnector;
use crate::message::Message;
use crate::secret_manager::{SecretManager, SecretType};

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
#[derive(Clone)]
pub struct ImapClient<A> {
    imap_domain: String,
    email: String,
    secret_manager: Option<A>,
    polling_time: Duration,
}

impl<A> Default for ImapClient<A> {
    fn default() -> Self {
        Self {
            imap_domain: String::default(),
            email: String::default(),
            secret_manager: None,
            polling_time: Duration::ZERO,
        }
    }
}

impl<A: SecretManager> ImapClient<A> {
    pub fn domain(mut self, value: impl Into<String>) -> Self {
        self.imap_domain = value.into();
        self
    }

    pub fn email(mut self, value: impl Into<String>) -> Self {
        self.email = value.into();
        self
    }

    pub fn secret_manager(mut self, secret_manager: A) -> Self {
        self.secret_manager = Some(secret_manager);
        self
    }

    pub fn polling_time(mut self, duration: Duration) -> Self {
        self.polling_time = duration;
        self
    }

    async fn connect(&self) -> Result<Session<TlsStream<TcpStream>>, Error> {
        let tls = TlsConnector::builder().build().unwrap();
        let client = imap::connect(
            (self.imap_domain.as_str(), 993),
            self.imap_domain.as_str(),
            &tls,
        )?;

        let secret_manager = self.secret_manager.as_ref().unwrap();
        match secret_manager.secret_type() {
            SecretType::Password => client
                .login(&self.email, secret_manager.secret().await)
                .map_err(|e| e.0),
            SecretType::AccessToken => {
                let gmail_auth = GmailOAuth2 {
                    user: self.email.clone(),
                    access_token: secret_manager.secret().await,
                };
                client.authenticate("XOAUTH2", &gmail_auth).map_err(|e| e.0)
            }
        }
    }
}

#[async_trait]
impl<A: SecretManager + Sync + Send + 'static> InputConnector for ImapClient<A> {
    async fn run(mut self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel> {
        let mut session = self.connect().await.unwrap();
        loop {
            std::thread::sleep(self.polling_time);

            match read_inbox(&mut session) {
                Ok(Some(message)) => sender.send(message).await?,
                Ok(None) => (),
                Err(err) => {
                    log::warn!("{}", err);
                    session = match self.connect().await {
                        Ok(session) => {
                            log::info!("Connection restored");
                            session
                        }
                        Err(Error::No(msg))
                            if msg.contains("AUTHENTICATIONFAILED")
                                && self.secret_manager.as_mut().unwrap().secret_type()
                                    == SecretType::AccessToken =>
                        {
                            log::trace!("expired access token, refreshing...");
                            self.secret_manager.as_mut().unwrap().refresh().await;
                            continue;
                        }
                        Err(err) => {
                            log::error!("{}", err);
                            continue;
                        }
                    }
                }
            }
        }
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
