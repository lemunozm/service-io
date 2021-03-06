use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::{InputConnector, OutputConnector};
use crate::message::Message;

use async_trait::async_trait;
use tokio::sync::mpsc;

#[async_trait]
impl InputConnector for mpsc::Receiver<Message> {
    async fn run(mut self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel> {
        loop {
            match self.recv().await {
                Some(message) => sender.send(message).await?,
                None => break Ok(()),
            };
        }
    }
}

#[async_trait]
impl OutputConnector for mpsc::Sender<Message> {
    async fn run(self: Box<Self>, mut receiver: Receiver) -> Result<(), ClosedChannel> {
        loop {
            let message = receiver.recv().await?;
            if self.send(message).await.is_err() {
                break Ok(());
            }
        }
    }
}
