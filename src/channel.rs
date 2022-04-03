//! Channels used to connect inputs with services and services with outputs.
//! It basically wraps a [`tokio::sync::mpsc`] for easy management inside input/output/services
//! implementations.

use crate::message::Message;

use tokio::sync::mpsc;

/// Error indicating that the channel was closed.
#[derive(Debug)]
pub struct ClosedChannel;

/// Sender side of the channel.
/// It basically wraps a [`tokio::sync::mpsc::Sender`] for easy management inside input/output/services
/// implementations.
#[derive(Clone)]
pub struct Sender(pub(crate) mpsc::Sender<Message>);

impl Sender {
    /// Send asynchronously a message.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Sender::send()`] with an specific
    /// mapped error.
    pub async fn send(&self, message: Message) -> Result<(), ClosedChannel> {
        self.0.send(message).await.map_err(|_| ClosedChannel)
    }

    /// Send a message.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Sender::blocking_send()`] with an
    /// specific mapped error.
    pub fn blocking_send(&self, message: Message) -> Result<(), ClosedChannel> {
        self.0.blocking_send(message).map_err(|_| ClosedChannel)
    }
}

/// Receiver side of the channel.
/// It basically wraps a [`tokio::sync::mpsc::Receiver`] for easy management inside input/output/services
/// implementations.
pub struct Receiver(pub(crate) mpsc::Receiver<Message>);

impl Receiver {
    /// Receive asynchronously a message.
    ///
    /// This method is a wrapper over [`tokio::sync::mpsc::Receiver::recv()`] with an specific
    /// mapped error.
    pub async fn recv(&mut self) -> Result<Message, ClosedChannel> {
        self.0.recv().await.ok_or(ClosedChannel)
    }
}
