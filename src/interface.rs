use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::message::Message;

use async_trait::async_trait;

#[async_trait]
pub trait InputConnector {
    async fn run(self: Box<Self>, sender: Sender<Message>) -> Result<(), ClosedChannel>;
}

#[async_trait]
pub trait OutputConnector {
    async fn run(self: Box<Self>, receiver: Receiver<Message>) -> Result<(), ClosedChannel>;
}

#[async_trait]
pub trait Service {
    async fn run(
        self: Box<Self>,
        input: Receiver<Message>,
        output: Sender<Message>,
    ) -> Result<(), ClosedChannel>;
}
