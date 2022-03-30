use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;
use crate::message::Message;

use async_trait::async_trait;

#[derive(Clone)]
pub struct Echo;

#[async_trait]
impl Service for Echo {
    async fn run(
        self: Box<Self>,
        mut input: Receiver<Message>,
        output: Sender<Message>,
    ) -> Result<(), ClosedChannel> {
        loop {
            let message = input.recv().await?;
            output.send(message).await?;
        }
    }
}
