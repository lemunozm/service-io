//! Traits for building [`InputConnector`], [`OutputConnector`], and [`Service`]
//!
//! [`InputConnector`]: interface::InputConnector
//! [`OutputConnector`]: interface::OutputConnector
//! [`Service`]: interface::Service

use crate::channel::{ClosedChannel, Receiver, Sender};

use async_trait::async_trait;

/// Trait to implement an input connector.
/// An input connector is in change of creating [`Message`] and sending to the services.
///
/// If the sender return a [`ClosedChannel`] error, it is expected to propagate this error.
///
/// See default implementations in [`connectors`]
///
/// Do not forget to add the [`mod@async_trait`] crate when implement this trait
///
/// [`connectors`]: crate::connectors
#[async_trait]
pub trait InputConnector {
    async fn run(self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel>;
}

/// Trait to implement an output connector.
/// An output connector is in change of delivering the [`Message`] created by the service.
///
/// If the receiver return a [`ClosedChannel`] error, it is expected to propagate this error.
///
/// See default implementations in [`connectors`]
///
/// Do not forget to add the [`mod@async_trait`] crate when implement this trait
///
/// [`connectors`]: crate::connectors
#[async_trait]
pub trait OutputConnector {
    async fn run(self: Box<Self>, receiver: Receiver) -> Result<(), ClosedChannel>;
}

/// Trait to implement a Service.
/// A Service is an entity that processes input messages asynchronously and send output messages
/// asynchronously.
///
/// If both, sender or receiver return a [`ClosedChannel`] error,
/// it is expected to propagate this error.
///
/// See default implementations in [`services`]
///
/// Do not forget to add the [`mod@async_trait`] crate when implement this trait
///
/// [`services`]: crate::services
#[async_trait]
pub trait Service {
    async fn run(self: Box<Self>, input: Receiver, output: Sender) -> Result<(), ClosedChannel>;
}
