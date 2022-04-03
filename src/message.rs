//! Common data shared among input/output/services and utilities related to it.

use std::collections::HashMap;

/// Common data shared among input/output/services.
/// This is the language `service-io` talk.
/// Each input/output/service understand this structure.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Message {
    /// The user this message is related to.
    /// If the message is in the input side, this user means the originator of the message.
    /// If the message is in the output side, this user means the recipient of the message.
    pub user: String,

    /// The service name this message going to/come from.
    /// The value of this field should match to any name used for register services.
    ///
    /// See also: [`Engine::add_service()`]
    ///
    /// [`Engine::add_service()`]: crate::engine::Engine::add_service()
    pub service_name: String,

    /// Arguments of the message.
    /// Each service implementation will understand these values in their own way.
    pub args: Vec<String>,

    /// Main body of the message.
    /// Each service implementation will understand this value in their own way.
    pub body: String,

    /// Attached content of the message.
    /// Each service implementation will understand these values in their own way.
    pub attached_data: HashMap<String, Vec<u8>>,
}

impl Message {
    /// Sugar to perform a response of a received message.
    /// Creates an empty message with same [`Message::user`]
    /// and [`Message::service_name`] as the passed message.
    pub fn response(message: &Message) -> Message {
        Message {
            user: message.user.clone(),
            service_name: message.service_name.clone(),
            ..Default::default()
        }
    }

    /// Set a user for the message
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = user.into();
        self
    }

    /// Set a service name for the message
    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = service_name.into();
        self
    }

    /// Set args for the message
    pub fn args<S: Into<String>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        self.args = args.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set a body for the message
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// Set attached data for the message
    pub fn attach<S: Into<String>>(
        mut self,
        attached: impl IntoIterator<Item = (S, Vec<u8>)>,
    ) -> Self {
        self.attached_data = attached
            .into_iter()
            .map(|(name, data)| (name.into(), data))
            .collect();
        self
    }
}

/// Utilities related to the `Message`
pub mod util {
    use super::Message;

    /// Modify the [`Message::service_name`] value to make the first letter lowercase.
    ///
    /// This utility can be used in [`Engine::map_input()`] to send always
    /// a first letter lowercase version of the service_name to the engine to make the correct
    /// service match.
    ///
    /// This is useful because some users could specify a first capital letter without realizing
    /// (usually in email clients where the first letter is uppercase by default).
    /// If the service_name is registered with lowercase, their message will not match.
    ///
    /// [`Engine::map_input()`]: crate::engine::Engine::map_input()
    pub fn service_name_first_char_to_lowercase(mut message: Message) -> Message {
        let mut chars = message.service_name.chars();
        message.service_name = match chars.next() {
            Some(first_letter) => first_letter.to_lowercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        };
        message
    }
}
