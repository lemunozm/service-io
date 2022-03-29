use crate::channel::{Receiver, Sender};
use crate::interface::{InputConnector, Message, OutputConnector, Service};

use tokio::sync::{mpsc, mpsc::error::SendError};

use std::collections::HashMap;
use std::collections::HashSet;

struct ServiceConfig {
    name: String,
    builder: Box<dyn Fn() -> Box<dyn Service + Send> + Send>,
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
        service: impl Service + Send + Clone + 'static,
    ) -> Engine {
        self.service_configs.push(ServiceConfig {
            name: name.into(),
            builder: Box::new(move || Box::new(service.clone())),
            whitelist: None,
        });
        self
    }

    pub fn add_service_for<S: Into<String>>(
        mut self,
        name: impl Into<String>,
        service: impl Service + Send + Clone + 'static,
        whitelist: impl IntoIterator<Item = S>,
    ) -> Engine {
        self.service_configs.push(ServiceConfig {
            name: name.into(),
            builder: Box::new(move || Box::new(service.clone())),
            whitelist: Some(whitelist.into_iter().map(|s| s.into()).collect()),
        });
        self
    }

    pub async fn run(self) {
        log::info!("Initializing engine...");

        let (input_sender, mut input_receiver) = mpsc::channel(32);
        tokio::spawn(async move {
            let task =
                tokio::spawn(async move { self.input.unwrap().run(Sender(input_sender)).await });

            log::info!("Loading input connector");

            match task.await {
                Ok(Ok(())) => log::info!("Input connector down (finished)"),
                Ok(Err(_)) => log::info!("Input connector down (disconnected)"),
                Err(_) => log::error!("Input connector down (panicked)"),
            }
        });

        let (output_sender, output_receiver) = mpsc::channel(32);
        let mut output_task = tokio::spawn(async move {
            let task =
                tokio::spawn(
                    async move { self.output.unwrap().run(Receiver(output_receiver)).await },
                );

            log::info!("Loading output connector");

            match task.await {
                Ok(Ok(())) => log::info!("Output connector down (finished)"),
                Ok(Err(_)) => log::info!("Output connector down (disconnected)"),
                Err(_) => log::error!("Output connector down (panicked)"),
            }
        });

        let services: HashMap<String, ServiceHandle> = self
            .service_configs
            .into_iter()
            .map(|config| {
                let (input_sender, input_receiver) = mpsc::channel(32);
                let output_sender = output_sender.clone();
                let service = (config.builder)();
                let service_name = config.name.clone();
                tokio::spawn(async move {
                    let task = tokio::spawn(async move {
                        service
                            .run(Receiver(input_receiver), Sender(output_sender))
                            .await
                    });

                    log::info!("Loading service '{}'", service_name);

                    match task.await {
                        Ok(Ok(())) => log::info!("Service '{}' down (finished)", service_name),
                        Ok(Err(_)) => log::info!("Service '{}' down (disconnected)", service_name),
                        Err(_) => log::error!("Service '{}' down (panicked)", service_name),
                    }
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

        drop(output_sender);

        loop {
            tokio::select! {
                input_message = input_receiver.recv() => {
                    let input_message = input_message.unwrap();
                    if let Some(handle) = services.get(&input_message.service) {
                        let allowed = match &handle.whitelist {
                            Some(whitelist) => whitelist.contains(&input_message.user),
                            None => true,
                        };

                        if allowed {
                            if let Err(SendError(message)) = handle.input_sender.send(input_message).await {
                                log::error!("Drop message for removed service '{}'", message.service);
                            }
                        }
                    }
                }
                _ = &mut output_task => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ClosedChannel;

    use async_trait::async_trait;
    use tokio::time::timeout;

    use std::time::Duration;

    #[derive(Clone)]
    pub struct EchoOnce;

    #[async_trait]
    impl Service for EchoOnce {
        async fn run(
            self: Box<Self>,
            mut input: Receiver<Message>,
            output: Sender<Message>,
        ) -> Result<(), ClosedChannel> {
            let message = input.recv().await?;
            output.send(message).await
        }
    }

    fn build_message(user: &str, service: &str) -> Message {
        Message {
            user: user.into(),
            service: service.into(),
            args: vec!["arg0".into(), "arg1".into()],
            body: "abcd".into(),
            files: [
                ("file1".to_string(), b"1234".to_vec()),
                ("file2".to_string(), b"5678".to_vec()),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[tokio::test]
    async fn echo() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        let task = tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service("s-echo", EchoOnce)
                .run()
                .await;
        });

        let message = build_message("user_0", "s-echo");
        input_sender.send(message.clone()).await.unwrap();
        assert_eq!(Some(message), output_receiver.recv().await);

        task.await.unwrap();
    }

    #[tokio::test]
    async fn no_services() {
        let (_input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut _output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .run()
                .await;
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn service_not_found() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service("s-echo", EchoOnce)
                .run()
                .await;
        });

        let message = build_message("user_0", "unknown");
        input_sender.send(message.clone()).await.unwrap();
        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn whitelist() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        let task = tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service_for("s-echo", EchoOnce, ["user_allowed"])
                .run()
                .await;
        });

        let message = build_message("user_not_allowed", "s-echo");
        input_sender.send(message.clone()).await.unwrap();
        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());

        let message = build_message("user_allowed", "s-echo");
        input_sender.send(message.clone()).await.unwrap();
        assert_eq!(Some(message), output_receiver.recv().await);

        task.await.unwrap();
    }
}
