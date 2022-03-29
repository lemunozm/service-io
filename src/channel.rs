use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub struct ClosedChannel;

#[derive(Clone)]
pub struct Sender<T>(pub(crate) mpsc::Sender<T>);

impl<T> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), ClosedChannel> {
        self.0.send(value).await.map_err(|_| ClosedChannel)
    }

    pub async fn blocking_send(&self, value: T) -> Result<(), ClosedChannel> {
        self.0.blocking_send(value).map_err(|_| ClosedChannel)
    }
}

pub struct Receiver<T>(pub(crate) Arc<Mutex<mpsc::Receiver<T>>>);

impl<T> Receiver<T> {
    pub async fn recv(&mut self) -> Result<T, ClosedChannel> {
        self.0.lock().await.recv().await.ok_or(ClosedChannel)
    }
}