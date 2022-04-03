//! Channels used to connect inputs with services and services with outputs.
//! It basically wraps a [`tokio::sync::mpsc`] for easy management inside input/output/services
//! implementations.

use tokio::sync::mpsc;

/// Error indicating that the channel was closed.
#[derive(Debug)]
pub struct ClosedChannel;

/// Sender side of the channel.
#[derive(Clone)]
pub struct Sender<T>(pub(crate) mpsc::Sender<T>);

impl<T> Sender<T> {
    /// Send asynchronously an event.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Sender::send()`] with an specific
    /// mapped error.
    pub async fn send(&self, value: T) -> Result<(), ClosedChannel> {
        self.0.send(value).await.map_err(|_| ClosedChannel)
    }

    /// Send an event.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Sender::blocking_send()`] with an
    /// specific mapped error.
    pub fn blocking_send(&self, value: T) -> Result<(), ClosedChannel> {
        self.0.blocking_send(value).map_err(|_| ClosedChannel)
    }
}

/// Receiver side of the channel.
pub struct Receiver<T>(pub(crate) mpsc::Receiver<T>);

impl<T> Receiver<T> {
    /// Receive asynchronously an event.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Receiver::recv()`] with an specific
    /// mapped error.
    pub async fn recv(&mut self) -> Result<T, ClosedChannel> {
        self.0.recv().await.ok_or(ClosedChannel)
    }
}
