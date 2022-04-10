use crate::channel::{ClosedChannel, Receiver, Sender};
use crate::interface::Service;

use async_trait::async_trait;

/// Returns the same received message without modifications
pub struct Echo;

#[async_trait]
impl Service for Echo {
    async fn run(
        self: Box<Self>,
        mut input: Receiver,
        output: Sender,
    ) -> Result<(), ClosedChannel> {
        loop {
            let message = input.recv().await?;
            output.send(message).await?;
        }
    }
}
