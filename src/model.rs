use std::rc::Rc;

use crate::navicula::traits::EnvironmentType;

#[derive(Default)]
pub struct Environment {
    chats: Rc<Vec<Chat>>,
}

impl Environment {
    pub fn new(chats: Vec<Chat>) -> Self {
        Self {
            chats: Rc::new(chats),
        }
    }

    pub fn chats(&self) -> Rc<Vec<Chat>> {
        self.chats.clone()
    }
}

impl EnvironmentType for Environment {
    type AppEvent = AppEvent;
}

#[derive(Clone, Debug)]
pub struct Chat {
    pub id: u64,
    pub with: String,
    pub messages: Vec<Message>,
}

impl PartialEq for Chat {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Chat {}

#[derive(Clone, Debug)]
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
                id: 1,
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
                id: 2,
                with: "Hans".to_string(),
                messages: vec![
                    Message::Send("Whats up".to_string()),
                    Message::Received("Bored".to_string()),
                    Message::Received("You?".to_string()),
                    Message::Send("Bored!".to_string()),
                ],
            },
            Chat {
                id: 3,
                with: "Carl".to_string(),
                messages: vec![Message::Received("Hey!".to_string())],
            },
        ]
    }
}
