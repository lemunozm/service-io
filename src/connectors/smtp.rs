use crate::channel::{ClosedChannel, Receiver};
use crate::interface::OutputConnector;
use crate::message::Message;
use crate::secret_manager::{SecretManager, SecretType};
use crate::util::IntoOption;

use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use async_trait::async_trait;

/// Output connector that acts as a SMTP client
/// The service sends emails to the SMTP server.
/// The service name is added as first word of the subject following by space.
/// The arguments are added as a words to the subject separated by spaces.
#[derive(Clone)]
pub struct SmtpClient<A> {
    smtp_domain: String,
    email: String,
    secret_manager: Option<A>,
    sender_name: Option<String>,
}

impl<A> Default for SmtpClient<A> {
    fn default() -> Self {
        Self {
            smtp_domain: String::default(),
            email: String::default(),
            secret_manager: None,
            sender_name: None,
        }
    }
}

impl<A: SecretManager> SmtpClient<A> {
    pub fn domain(mut self, value: impl Into<String>) -> Self {
        self.smtp_domain = value.into();
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

    /// Name alias for the email
    pub fn sender_name(mut self, value: impl IntoOption<String>) -> Self {
        self.sender_name = value.into_some();
        self
    }
}

#[async_trait]
impl<A: SecretManager + Sync + Send> OutputConnector for SmtpClient<A> {
    async fn run(mut self: Box<Self>, mut receiver: Receiver) -> Result<(), ClosedChannel> {
        let address = self.email.parse::<Address>().unwrap();
        let user = address.user().to_string();
        let mut secret_manager = self.secret_manager.unwrap();
        let from = Mailbox::new(self.sender_name, address);

        loop {
            let message = receiver.recv().await?;
            if let Some(email) = message_to_email(message, from.clone()) {
                loop {
                    let mailer =
                        AsyncSmtpTransport::<Tokio1Executor>::relay(self.smtp_domain.as_ref())
                            .unwrap()
                            .authentication(vec![match secret_manager.secret_type() {
                                SecretType::Password => Mechanism::Login,
                                SecretType::AccessToken => Mechanism::Xoauth2,
                            }])
                            .credentials(Credentials::new(
                                user.clone(),
                                secret_manager.secret().await,
                            ))
                            .build();

                    match mailer.send(email.clone()).await {
                        Ok(_) => break,
                        Err(err)
                            if format!("{}", err).contains("challenge")
                                && secret_manager.secret_type() == SecretType::AccessToken =>
                        {
                            log::trace!("expired access token, refreshing...");
                            secret_manager.refresh().await;
                            continue;
                        }
                        Err(err) => {
                            log::error!("Sending error: {}", err);
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn message_to_email(message: Message, from: Mailbox) -> Option<lettre::Message> {
    let to_address = message
        .user
        .parse::<Address>()
        .map_err(|err| log::error!("{}", err))
        .ok()?;

    let single_parts = message
        .attached_data
        .into_iter()
        .map(|(filename, filebody)| {
            Some(
                Attachment::new(filename).body(
                    filebody,
                    ContentType::parse("application/octet-stream")
                        .map_err(|err| log::error!("{}", err))
                        .ok()?,
                ),
            )
        })
        .collect::<Vec<_>>();

    let mut multipart = MultiPart::alternative().singlepart(SinglePart::plain(message.body));
    for single in single_parts {
        multipart = multipart.singlepart(single?);
    }

    let subject = message.args.join(" ");

    lettre::Message::builder()
        .from(from)
        .to(Mailbox::new(None, to_address))
        .subject(format!("{} {}", message.service_name, subject))
        .multipart(multipart)
        .map_err(|err| log::error!("{}", err))
        .ok()
}
