use crate::util::IntoOption;

use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, SmtpTransport, Transport};

use std::fmt;

#[derive(Default)]
pub struct SmtpClient {
    smtp_domain: String,
    email: String,
    password: String,
    sender_name: Option<String>,
}

impl SmtpClient {
    pub fn domain(mut self, value: impl Into<String>) -> Self {
        self.smtp_domain = value.into();
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

    pub fn sender_name(mut self, value: impl IntoOption<String>) -> Self {
        self.sender_name = value.into_some();
        self
    }

    pub fn build(self) -> SmtpClient {
        let address = self.email.parse::<Address>().unwrap();
        let user = address.user().to_string();
        let credentials = Credentials::new(user, self.password.into());

        SmtpClient {
            from: Mailbox::new(self.sender_name, address),
            mailer: SmtpTransport::relay(self.smtp_domain.as_ref())
                .unwrap()
                .credentials(credentials)
                .build(),
        }
    }
}

pub struct SmtpClient {
    from: Mailbox,
    mailer: SmtpTransport,
}

impl SmtpClient {
    pub fn builder() -> SmtpClientBuilder {
        SmtpClientBuilder::default()
    }

    pub fn send_email(
        &mut self,
        to_email: &str,
        subject: &str,
        body: impl Into<String>,
        attachments: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<(), SmtpClientError> {
        let to_address = to_email.parse::<Address>().unwrap();

        let single_parts = attachments
            .into_iter()
            .map(|(filename, filebody)| {
                Attachment::new(filename).body(
                    filebody,
                    ContentType::parse("application/octet-stream").unwrap(),
                )
            })
            .collect::<Vec<_>>();

        let mut multipart = MultiPart::alternative().singlepart(SinglePart::plain(body.into()));
        for single in single_parts {
            multipart = multipart.singlepart(single);
        }

        let email = lettre::Message::builder()
            .from(self.from.clone())
            .to(Mailbox::new(None, to_address))
            .subject(subject)
            .multipart(multipart)
            .map_err(|_| SmtpClientError::EmailFormat)?;

        self.mailer
            .send(&email)
            .map(|_| ())
            .map_err(|_| SmtpClientError::Connection)
    }
}

#[derive(Debug)]
pub enum SmtpClientError {
    Connection,
    EmailFormat,
}

impl fmt::Display for SmtpClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SmtpClientError {}
