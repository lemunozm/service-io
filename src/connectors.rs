mod user_stdin;
pub use user_stdin::UserStdin;

mod debug_stdout;
pub use debug_stdout::DebugStdout;

use crate::interface::{InputConnector, Message, OutputConnector};

use tokio::sync::mpsc;

use async_trait::async_trait;

// ================================
//          DEFAULT INPUTS
// ================================
#[async_trait]
impl InputConnector for mpsc::Receiver<Message> {
    async fn run(mut self: Box<Self>, sender: mpsc::Sender<Message>) {
        loop {
            let message = self.recv().await.unwrap();
            sender.send(message).await.unwrap()
        }
    }
}

// ================================
//          DEFAULT OUTPUTS
// ================================
#[async_trait]
impl OutputConnector for mpsc::Sender<Message> {
    async fn run(self: Box<Self>, mut receiver: mpsc::Receiver<Message>) {
        loop {
            let message = receiver.recv().await.unwrap();
            self.send(message).await.unwrap()
        }
    }
}
