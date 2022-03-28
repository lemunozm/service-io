use crate::interface::{Message, Service};

use tokio::sync::mpsc;

use async_trait::async_trait;

pub struct Echo;

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
