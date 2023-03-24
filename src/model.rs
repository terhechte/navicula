use crate::navicula::traits::EnvironmentType;

#[derive(Default)]
pub struct Environment {
    pub chats: Vec<Chat>,
}

impl Environment {
    pub fn chats(&self) -> &[Chat] {
        &self.chats
    }
}

impl EnvironmentType for Environment {
    type AppEvent = AppEvent;
}

pub struct Chat {
    pub with: String,
    pub messages: Vec<Message>,
}

pub enum Message {
    Send(String),
    Received(String),
}

pub enum AppEvent {
    Something,
}

pub mod mock {
    use super::{Chat, Message};
    pub fn chats() -> Vec<Chat> {
        vec![
            Chat {
                with: "Klaus".to_string(),
                messages: vec![
                    Message::Send("Hey, how are you".to_string()),
                    Message::Received("I'm good".to_string()),
                    Message::Received("You?".to_string()),
                    Message::Send("Top".to_string()),
                    Message::Send("You up for Baseball today?".to_string()),
                    Message::Received("I'm game".to_string()),
                ],
            },
            Chat {
                with: "Hans".to_string(),
                messages: vec![
                    Message::Send("Whats up".to_string()),
                    Message::Received("Bored".to_string()),
                    Message::Received("You?".to_string()),
                    Message::Send("Bored!".to_string()),
                ],
            },
            Chat {
                with: "Hans".to_string(),
                messages: vec![Message::Received("Hey!".to_string())],
            },
        ]
    }
}
