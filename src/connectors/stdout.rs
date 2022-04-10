use crate::channel::{ClosedChannel, Receiver};
use crate::interface::OutputConnector;

use tokio::io::AsyncWriteExt;

use async_trait::async_trait;

/// Print the message to the stdout.
pub struct DebugStdout;

#[async_trait]
impl OutputConnector for DebugStdout {
    async fn run(mut self: Box<Self>, mut receiver: Receiver) -> Result<(), ClosedChannel> {
        loop {
            let message = receiver.recv().await?;
            tokio::io::stdout()
                .write_all(format!("{:#?}\n", message).as_bytes())
                .await
                .unwrap();
        }
    }
}
