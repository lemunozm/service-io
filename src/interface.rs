use tokio::sync::mpsc;

use async_trait::async_trait;

use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Message {
    pub user: String,
    pub service: String,
    pub args: Vec<String>,
    pub body: String,
    pub files: HashMap<String, Vec<u8>>,
}

#[async_trait]
pub trait InputConnector {
    async fn run(self: Box<Self>, sender: mpsc::Sender<Message>);
}

#[async_trait]
pub trait OutputConnector {
    async fn run(self: Box<Self>, receiver: mpsc::Receiver<Message>);
}

#[async_trait]
pub trait Service {
    async fn run(self: Box<Self>, input: mpsc::Receiver<Message>, output: mpsc::Sender<Message>);
}
