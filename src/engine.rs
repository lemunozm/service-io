use crate::interface::{InputConnector, Message, OutputConnector, Service};

use tokio::sync::mpsc;

use std::collections::HashMap;
use std::collections::HashSet;

struct ServiceConfig {
    name: String,
    service: Box<dyn Service + Send>,
    whitelist: Option<HashSet<String>>,
}

struct ServiceHandle {
    whitelist: Option<HashSet<String>>,
    input_sender: mpsc::Sender<Message>,
}

#[derive(Default)]
pub struct Engine {
    input: Option<Box<dyn InputConnector + Send>>,
    output: Option<Box<dyn OutputConnector + Send>>,
    service_configs: Vec<ServiceConfig>,
}

impl Engine {
    pub fn input(mut self, input: impl InputConnector + Send + 'static) -> Engine {
        self.input = Some(Box::new(input));
        self
    }

    pub fn output(mut self, output: impl OutputConnector + Send + 'static) -> Engine {
        self.output = Some(Box::new(output));
        self
    }

    pub fn add_service(
        mut self,
        name: impl Into<String>,
        service: impl Service + Send + 'static,
    ) -> Engine {
        self.service_configs.push(ServiceConfig {
            name: name.into(),
            service: Box::new(service),
            whitelist: None,
        });
        self
    }

    pub fn add_service_for<S: Into<String>>(
        mut self,
        name: impl Into<String>,
        service: impl Service + Send + 'static,
        whitelist: impl IntoIterator<Item = S>,
    ) -> Engine {
        self.service_configs.push(ServiceConfig {
            name: name.into(),
            service: Box::new(service),
            whitelist: Some(whitelist.into_iter().map(|s| s.into()).collect()),
        });
        self
    }

    pub async fn run(self) {
        let (input_sender, mut input_receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            self.input.unwrap().run(input_sender).await;
        });

        let (output_sender, output_receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            self.output.unwrap().run(output_receiver).await;
        });

        let services: HashMap<String, ServiceHandle> = self
            .service_configs
            .into_iter()
            .map(|config| {
                let (input_sender, input_receiver) = mpsc::channel(32);
                let output_sender = output_sender.clone();
                tokio::spawn(async move {
                    config.service.run(input_receiver, output_sender).await;
                });

                (
                    config.name,
                    ServiceHandle {
                        whitelist: config.whitelist,
                        input_sender,
                    },
                )
            })
            .collect();

        loop {
            let input_message = input_receiver.recv().await.unwrap();
            if let Some(handle) = services.get(&input_message.service) {
                let allowed = match &handle.whitelist {
                    Some(whitelist) => whitelist.contains(&input_message.user),
                    None => true,
                };

                if allowed {
                    handle.input_sender.send(input_message).await.unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;

    #[async_trait]
    impl InputConnector for mpsc::Receiver<Message> {
        async fn run(mut self: Box<Self>, sender: mpsc::Sender<Message>) {
            loop {
                let message = self.recv().await.unwrap();
                sender.send(message).await.unwrap()
            }
        }
    }

    #[async_trait]
    impl OutputConnector for mpsc::Sender<Message> {
        async fn run(self: Box<Self>, mut receiver: mpsc::Receiver<Message>) {
            loop {
                let message = receiver.recv().await.unwrap();
                self.send(message).await.unwrap()
            }
        }
    }

    struct Echo;

    #[async_trait]
    impl Service for Echo {
        async fn run(
            self: Box<Self>,
            mut input: mpsc::Receiver<Message>,
            output: mpsc::Sender<Message>,
        ) {
            loop {
                let message = input.recv().await.unwrap();
                output.send(message).await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn basic() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service("s-echo", Echo)
                .run()
                .await;
        });

        let input_message = Message {
            user: "user_0".into(),
            service: "s-echo".into(),
            args: vec!["arg0".into(), "arg1".into()],
            body: "abcd".into(),
            files: [
                ("file1".to_string(), b"1234".to_vec()),
                ("file2".to_string(), b"5678".to_vec()),
            ]
            .into_iter()
            .collect(),
        };

        input_sender.send(input_message.clone()).await.unwrap();
        let output_message = output_receiver.recv().await.unwrap();

        assert_eq!(input_message, output_message);
    }
}
