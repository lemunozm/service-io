use crate::interface::{InputConnector, Message, OutputConnector, Service};

use tokio::sync::{mpsc, RwLock};

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

struct ServiceConfig {
    name: String,
    builder: Box<dyn Fn() -> Box<dyn Service + Send> + Send>,
    whitelist: Option<HashSet<String>>,
}

struct ServiceHandle {
    whitelist: Option<HashSet<String>>,
    input_sender: mpsc::Sender<Message>,
    //input_sender: Arc<RwLock<mpsc::Sender<Message>>>,
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
                    let service = (config.builder)();
                    //loop {
                    let task = tokio::spawn(async move {
                        service.run(input_receiver, output_sender).await;
                    });

                    if task.await.is_ok() {
                        //break;
                    }
                    //}
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
    use crate::services::Echo;

    use async_trait::async_trait;
    use tokio::time::timeout;

    use std::time::Duration;

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

    #[derive(Clone)]
    struct Panic;

    #[async_trait]
    impl Service for Panic {
        async fn run(
            self: Box<Self>,
            _input: mpsc::Receiver<Message>,
            _output: mpsc::Sender<Message>,
        ) {
            panic!("The test service has panicked");
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

        let input_message = build_message("user_0", "s-echo");
        input_sender.send(input_message.clone()).await.unwrap();

        let output_message = output_receiver.recv().await.unwrap();
        assert_eq!(input_message, output_message);

        let input_message = build_message("user_0", "unknown");
        input_sender.send(input_message.clone()).await.unwrap();

        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn whitelist() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service_for("s-echo", Echo, ["user_0"])
                .run()
                .await;
        });

        let input_message = build_message("user_0", "s-echo");
        input_sender.send(input_message.clone()).await.unwrap();

        let output_message = output_receiver.recv().await.unwrap();
        assert_eq!(input_message, output_message);

        let input_message = build_message("user_1", "s-echo");
        input_sender.send(input_message.clone()).await.unwrap();

        assert!(timeout(Duration::from_millis(100), output_receiver.recv())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn surviving_to_panic() {
        let (input_sender, input_receiver) = mpsc::channel(32);
        let (output_sender, mut output_receiver) = mpsc::channel(32);

        tokio::spawn(async move {
            Engine::default()
                .input(input_receiver)
                .output(output_sender)
                .add_service("s-panic", Panic)
                .add_service("s-echo", Echo)
                .run()
                .await;
        });

        let input_message = build_message("user_0", "s-echo");
        input_sender.send(input_message.clone()).await.unwrap();

        let output_message = output_receiver.recv().await.unwrap();
        assert_eq!(input_message, output_message);
    }
}
