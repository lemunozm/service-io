//! Traits for building [`InputConnector`], [`OutputConnector`], and [`Service`]
//!
//! [`InputConnector`]: interface::InputConnector
//! [`OutputConnector`]: interface::OutputConnector
//! [`Service`]: interface::Service

use crate::channel::{ClosedChannel, Receiver, Sender};

use async_trait::async_trait;

/// Implement an input connector.
/// An input connector is in change of creating [`Message`] and sending asynchronously
/// to the services.
///
/// If the sender return a [`ClosedChannel`] error, it is expected to propagate this error.
///
/// See default implementations in [`connectors`]
///
/// Do not forget to add the [`mod@async_trait`] crate when implement this trait
///
/// [`connectors`]: crate::connectors
/// [`Message`]: crate::message::Message
///
/// # Example
/// ```rust
/// use service_io::interface::{InputConnector};
/// use service_io::channel::{ClosedChannel, Sender};
/// use service_io::message::{Message};
///
/// use async_trait::async_trait;
///
/// struct MyInput;
///
/// #[async_trait]
/// impl InputConnector for MyInput {
///     async fn run(self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel> {
///          // Load phase
///          // ...
///          loop {
///              // Get the message from your implementation
///              // ...
///              let message = Message::default();
///
///              // Send the message to the service
///              sender.send(message).await?;
///          }
///     }
/// }
/// ```
#[async_trait]
pub trait InputConnector {
    async fn run(self: Box<Self>, sender: Sender) -> Result<(), ClosedChannel>;
}

/// Implement an output connector.
/// An output connector is waiting asynchronously for messages comming from the services
/// and deliver them.
///
/// If the receiver return a [`ClosedChannel`] error, it is expected to propagate this error.
///
/// See default implementations in [`connectors`]
///
/// Do not forget to add the [`mod@async_trait`] crate when implement this trait
///
/// [`connectors`]: crate::connectors
/// [`Message`]: crate::message::Message
///
/// # Example
/// ```rust
///
/// use service_io::interface::{OutputConnector};
/// use service_io::channel::{ClosedChannel, Receiver};
///
/// use async_trait::async_trait;
///
/// struct MyOutput;
///
/// #[async_trait]
/// impl OutputConnector for MyOutput {
///     async fn run(self: Box<Self>, mut receiver: Receiver) -> Result<(), ClosedChannel> {
///          // Load phase
///          // ...
///          loop {
///              // Get the message from the service...
///              let message = receiver.recv().await?;
///
///              // Do whatever your output impl must do. i.e send the message by email
///              // ...
///          }
///     }
/// }
/// ```
#[async_trait]
pub trait OutputConnector {
    async fn run(self: Box<Self>, receiver: Receiver) -> Result<(), ClosedChannel>;
}

/// Implement a service.
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
///
/// # Example
/// ```rust
/// use service_io::interface::{Service};
/// use service_io::channel::{ClosedChannel, Receiver, Sender};
///
/// use async_trait::async_trait;
///
/// struct MyService;
///
/// #[async_trait]
/// impl Service for MyService {
///     async fn run(self: Box<Self>, mut input: Receiver, output: Sender) -> Result<(), ClosedChannel> {
///          // Load phase
///          // ...
///          loop {
///              // Get the message from the input connector.
///              let message = input.recv().await?;
///
///              // Do whatever your service impl must do.
///              // ...
///
///              // Send the message to the output connector.
///              output.send(message).await?;
///          }
///     }
/// }
/// ```
#[async_trait]
pub trait Service {
    async fn run(self: Box<Self>, input: Receiver, output: Sender) -> Result<(), ClosedChannel>;
}
