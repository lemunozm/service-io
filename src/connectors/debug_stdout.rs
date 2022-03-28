use crate::interface::{Message, OutputConnector};

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use async_trait::async_trait;

pub struct DebugStdout;

#[async_trait]
impl OutputConnector for DebugStdout {
    async fn run(mut self: Box<Self>, mut receiver: mpsc::Receiver<Message>) {
        loop {
            let message = receiver.recv().await.unwrap();
            tokio::io::stdout()
                .write_all(format!("{:#?}\n", message).as_bytes())
                .await
                .unwrap();
        }
    }
}
