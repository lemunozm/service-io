//! Main entity of `service-io`.
//! Connects input, output, and services and run them.

use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::{InputConnector, OutputConnector, Service};
use crate::message::Message;

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
            let user = message.user.clone();
            let service_name = message.service_name.clone();
            let args = message.args.join(" ");
            match self.input_sender.send(message).await {
                Ok(()) => log::info!(
                    "Processing message from '{}' for service '{}' with args '{}'",
                    user,
                    service_name,
                    args
                ),
                Err(_) => log::warn!("Drop message for removed service '{}'", service_name),
            }
        } else {
            log::warn!(
                "Drop message for service '{}' not allowed for user '{}'",
                message.service_name,
                message.user,
            );
        }
    }
}

/// Main entity of service-io.
///
/// It defines the following schema that runs asynchronously: `Input -> n Services -> Output`
///
/// A message received by the [`InputConnector`] will be sent to a specific [`Service`] based on the
/// [`Message::service_name`]. The [`Service`] will process the message and optionally can sent any
/// number of output messages that will be delivered by the [`OutputConnector`].
#[derive(Default)]
pub struct Engine {
    input: Option<Box<dyn InputConnector + Send>>,
    output: Option<Box<dyn OutputConnector + Send>>,
    input_mapping: Option<Box<dyn Fn(Message) -> Message + Send>>,
    input_filtering: Option<Box<dyn Fn(&Message) -> bool + Send>>,
    service_configs: Vec<ServiceConfig>,
}

impl Engine {
    /// Set an input connector for this engine that will be run after calling [`Engine::run()`].
    ///
    /// Default connectors can be found in [`connectors`].
    /// This call is mandatory in order to run the engine.
    ///
    /// [`connectors`]: crate::connectors
    pub fn input(mut self, input: impl InputConnector + Send + 'static) -> Engine {
        self.input = Some(Box::new(input));
        self
    }

    /// Set an output connector for this engine that will be run after calling [`Engine::run()`].
    ///
    /// Default connectors can be found in [`connectors`].
    /// This call is mandatory in order to run the engine.
    ///
    /// [`connectors`]: crate::connectors
    pub fn output(mut self, output: impl OutputConnector + Send + 'static) -> Engine {
        self.output = Some(Box::new(output));
        self
    }

    /// Maps the message processed by the input connector into other message before checking the
    /// destination service the message is for.
    pub fn map_input(mut self, mapping: impl Fn(Message) -> Message + Send + 'static) -> Engine {
        self.input_mapping = Some(Box::new(mapping));
        self
    }

    /// Allow or disallow passing the message to the service based of the message itself.
    /// This filter method is applied just after the mapping method set by [`Engine::map_input`].
    pub fn filter_input(mut self, filtering: impl Fn(&Message) -> bool + Send + 'static) -> Engine {
        self.input_filtering = Some(Box::new(filtering));
        self
    }

    /// Add a service to the engine registered with a `name`. If the [`Message::service_name`] value
    /// matches with this `name`, the message will be redirected to the service.
    ///
    /// Note that the service will not run until you call [`Engine::run()`]
    ///
    /// Default services can be found in [`services`]
    ///
    /// [`services`]: crate::services
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

    /// Similar to [`Engine::add_service()`] but service only allow receive message for a whitelist of users.
    /// If the [`Message::user`] of the incoming message not belong to that list, the message is
    /// discarded.
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

    /// Run asynchronously the input, output and all services configured for this engine.
    /// The engine will run until all services finished or the input/output connector finalizes.
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
                    let message = match &self.input_mapping {
                        Some(map) => map(message),
                        None => message,
                    };

                    let allowed = match &self.input_filtering {
                        Some(filter) => filter(&message),
                        None => true,
                    };

                    if allowed {
                        match services.get(&message.service_name) {
                            Some(handle) => handle.process_message(message).await,
                            None => log::trace!(
                                "Drop Message from {} for unknown service '{}'",
                                message.user,
                                message.service_name
                            ),
                        }
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
    use crate::message::util;

    use async_trait::async_trait;
    use tokio::time::timeout;

    use std::time::Duration;

    #[derive(Clone)]
    pub struct EchoOnce;

    #[async_trait]
    impl Service for EchoOnce {
        async fn run(
            self: Box<Self>,
            mut input: Receiver,
            output: Sender,
        ) -> Result<(), ClosedChannel> {
            let message = input.recv().await?;
            output.send(message).await
        }
    }

    fn build_message(user: &str, service: &str) -> Message {
        Message {
            user: user.into(),
            service_name: service.into(),
            args: vec!["arg0".into(), "arg1".into()],
            body: "abcd".into(),
            attached_data: [
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
                .add_service("s-test", EchoOnce)
                .run()
                .await;
        });

        let message = build_message("user_0", "s-test");
        input_sender.send(message.clone()).await.unwrap();
        assert_eq!(Some(message), output_receiver.recv().await);

        task.await.unwrap();
    }

    #[tokio::test]
    async fn echo_with_input_mapping() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        let task = tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .map_input(util::service_name_first_char_to_lowercase)
                .add_service("s-test", EchoOnce)
                .run()
                .await;
        });

        let message = build_message("user_0", "S-test");
        input_sender.send(message.clone()).await.unwrap();
        assert_eq!(
            Some(util::service_name_first_char_to_lowercase(message)),
            output_receiver.recv().await
        );

        task.await.unwrap();
    }

    #[tokio::test]
    async fn echo_with_input_filtering() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .filter_input(|message| !message.service_name.starts_with("s-"))
                .add_service("s-test", EchoOnce)
                .run()
                .await;
        });

        let message = build_message("user_0", "s-test");
        input_sender.send(message.clone()).await.unwrap();
        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());
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
                .add_service("s-test", EchoOnce)
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
                .add_service_for("s-test", EchoOnce, ["user_allowed"])
                .run()
                .await;
        });

        let message = build_message("user_not_allowed", "s-test");
        input_sender.send(message.clone()).await.unwrap();
        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());

        let message = build_message("user_allowed", "s-test");
        input_sender.send(message.clone()).await.unwrap();
        assert_eq!(Some(message), output_receiver.recv().await);

        task.await.unwrap();
    }
}
