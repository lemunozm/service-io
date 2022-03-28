use crate::interface::{InputConnector, Message};

use tokio::sync::mpsc;

use async_trait::async_trait;

use std::io::{self, BufRead};

pub struct UserStdin<N>(pub N);

#[async_trait]
impl<N: Into<String> + Send> InputConnector for UserStdin<N> {
    async fn run(mut self: Box<Self>, sender: mpsc::Sender<Message>) {
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
                    service: service.into(),
                    args: words.map(|s| s.into()).collect(),
                    ..Default::default()
                };

                sender.send(message).await.unwrap();
            }
        }
    }
}