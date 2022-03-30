use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Message {
    pub user: String,
    pub service_name: String,
    pub args: Vec<String>,
    pub body: String,
    pub files: HashMap<String, Vec<u8>>,
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
