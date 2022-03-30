use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::{InputConnector, Message, OutputConnector, Service};

use tokio::{
    sync::mpsc,
    task::{JoinError, JoinHandle},
};

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

impl ServiceHandle {
    async fn process_message(&self, message: Message) {
        let allowed = match &self.whitelist {
            Some(whitelist) => whitelist.contains(&message.user),
            None => true,
        };

        if allowed {
            let service_name = message.service.clone();
            match self.input_sender.send(message).await {
                Ok(()) => log::info!("Processing message for service '{}'", service_name),
                Err(_) => log::warn!("Drop message for removed service '{}'", service_name),
            }
        } else {
            log::warn!(
                "Drop message for service '{}' not allowed for user '{}'",
                message.service,
                message.user,
            );
        }
    }
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
        log::info!("Initializing engine...");

        let (input_sender, mut input_receiver) = mpsc::channel(32);
        Self::load_input(self.input.unwrap(), input_sender);

        let (output_sender, output_receiver) = mpsc::channel(32);
        let mut output_task = Self::load_output(self.output.unwrap(), output_receiver);

        let services = Self::load_services(self.service_configs, output_sender);

        loop {
            tokio::select! {
                Some(message) = input_receiver.recv() => {
                    match services.get(&message.service) {
                        Some(handle) => handle.process_message(message).await,
                        None => log::warn!("Drop Message for unknown service '{}'", message.service),
                    }
                }
                _ = &mut output_task => break,
                else => break,
            }
        }
    }

    fn load_input(
        input: Box<dyn InputConnector + Send>,
        sender: mpsc::Sender<Message>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            log::info!("Loading input connector");

            let result = tokio::spawn(async move { input.run(Sender(sender)).await }).await;

            Self::log_join_result(result, "Input connector");
        })
    }

    fn load_output(
        output: Box<dyn OutputConnector + Send>,
        receiver: mpsc::Receiver<Message>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            log::info!("Loading output connector");

            let result = tokio::spawn(async move { output.run(Receiver(receiver)).await }).await;

            Self::log_join_result(result, "Output connector");
        })
    }

    fn load_service(
        service: Box<dyn Service + Send>,
        receiver: mpsc::Receiver<Message>,
        sender: mpsc::Sender<Message>,
        name: String,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            log::info!("Loading service '{}'", name);

            let result =
                tokio::spawn(async move { service.run(Receiver(receiver), Sender(sender)).await })
                    .await;

            Self::log_join_result(result, &format!("Service '{}'", name));
        })
    }

    fn load_services(
        configs: Vec<ServiceConfig>,
        output_sender: mpsc::Sender<Message>,
    ) -> HashMap<String, ServiceHandle> {
        let services = configs
            .into_iter()
            .map(|config| {
                let (input_sender, input_receiver) = mpsc::channel(32);
                let output_sender = output_sender.clone();
                let service_name = config.name.clone();

                Self::load_service(config.service, input_receiver, output_sender, service_name);

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

        services
    }

    fn log_join_result(result: Result<Result<(), ClosedChannel>, JoinError>, name: &str) {
        match result {
            Ok(Ok(())) => log::info!("{} down (finished)", name),
            Ok(Err(_)) => log::info!("{} down (disconnected)", name),
            Err(_) => log::error!("{} down (panicked)", name),
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
