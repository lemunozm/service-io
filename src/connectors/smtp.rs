use crate::channel::{ClosedChannel, Receiver};
use crate::interface::{Message, OutputConnector};
use crate::util::IntoOption;

use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use async_trait::async_trait;

#[derive(Default, Clone)]
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
}

#[async_trait]
impl OutputConnector for SmtpClient {
    async fn run(
        mut self: Box<Self>,
        mut receiver: Receiver<Message>,
    ) -> Result<(), ClosedChannel> {
        let address = self.email.parse::<Address>().unwrap();
        let user = address.user().to_string();
        let credentials = Credentials::new(user, self.password.into());

        let from = Mailbox::new(self.sender_name, address);
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(self.smtp_domain.as_ref())
            .unwrap()
            .credentials(credentials)
            .build();

        loop {
            let message = receiver.recv().await?;
            let email = message_to_email(message, from.clone());
            mailer.send(email).await.unwrap();
        }
    }
}

fn message_to_email(message: Message, from: Mailbox) -> lettre::Message {
    let to_address = message.user.parse::<Address>().unwrap();

    let single_parts = message
        .files
        .into_iter()
        .map(|(filename, filebody)| {
            Attachment::new(filename).body(
                filebody,
                ContentType::parse("application/octet-stream").unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let mut multipart = MultiPart::alternative().singlepart(SinglePart::plain(message.body));
    for single in single_parts {
        multipart = multipart.singlepart(single);
    }

    let subject = message.args.join(" ");

    lettre::Message::builder()
        .from(from)
        .to(Mailbox::new(None, to_address))
        .subject(format!("{} {}", message.service, subject))
        .multipart(multipart)
        .unwrap()
}
