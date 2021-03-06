use crate::channel::{ClosedChannel, Sender};
use crate::interface::InputConnector;
use crate::message::Message;

use async_trait::async_trait;

use std::io::{self, BufRead};

/// Reads a line from the stdin.
/// The service accepts a parameter that corresponds with the user.
/// The first word of the line is interpreted as the service name.
/// The following spaced-separated words are the arguments.
/// Neither body nor attach fields are populated.
pub struct UserStdin<N>(pub N);

#[async_trait]
impl<N: Into<String> + Send> InputConnector for UserStdin<N> {
    async fn run(mut self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel> {
        let user_name = self.0.into();
        loop {
            let line =
                tokio::task::spawn_blocking(|| io::stdin().lock().lines().next().unwrap().unwrap())
                    .await
                    .unwrap();

            let mut words = line.split_whitespace();
            if let Some(service) = words.next() {
                let message = Message {
                    user: user_name.clone(),
                    service_name: service.into(),
                    args: words.map(|s| s.into()).collect(),
                    ..Default::default()
                };

                sender.send(message).await?;
            }
        }
    }
}
