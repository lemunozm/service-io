use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Message {
    pub user: String,
    pub service_name: String,
    pub args: Vec<String>,
    pub body: String,
    pub files: HashMap<String, Vec<u8>>,
}

impl Message {
    pub fn response(message: &Message) -> Message {
        Message {
            user: message.user.clone(),
            service_name: message.service_name.clone(),
            ..Default::default()
        }
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = user.into();
        self
    }

    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = service_name.into();
        self
    }

    pub fn args<S: Into<String>>(mut self, args: impl IntoIterator<Item = S>) -> Self {
        self.args = args.into_iter().map(|s| s.into()).collect();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn files<S: Into<String>>(mut self, files: impl IntoIterator<Item = (S, Vec<u8>)>) -> Self {
        self.files = files
            .into_iter()
            .map(|(name, data)| (name.into(), data))
            .collect();
        self
    }
}

pub mod util {
    use super::Message;

    pub fn service_name_first_char_to_lowercase(mut message: Message) -> Message {
        let mut chars = message.service_name.chars();
        message.service_name = match chars.next() {
            Some(first_letter) => first_letter.to_lowercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        };
        message
    }
}
