use dioxus::prelude::*;

use crate::{
    model::Chat,
    navicula::{
        self,
        effect::Effect,
        traits::{ReducerContext, VviewStore},
        types::MessageContext,
    },
};

pub struct RootReducer {
    // environment: super::Environment,
}

// impl Default for RootReducer {
//     fn default() -> Self {
//         Self {
//             environment: Default::default(),
//         }
//     }
// }

#[derive(Clone, Debug)]
pub enum Message {
    Reload,
}

// impl IntoAction<Action> for Message {
//     fn into_action(self) -> Action {
//         Action::Message(self)
//     }
// }

#[derive(Clone)]
pub enum DelegateMessage {
    Closed,
}

#[derive(Clone, Debug)]
pub enum Action {
    Initial,
    Load,
    Reload,
    CreateChat,
    Selected(u64),
    ClosedMessage,
}

pub struct State {
    counter: usize,
    selected: Option<Chat>,
}

impl State {
    pub fn new() -> Self {
        State {
            counter: 0,
            selected: None,
        }
    }
}

impl navicula::traits::Reducer for RootReducer {
    type Message = Message;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    // Provide the environment
    // fn environment(&self) -> &Self::Environment {
    //     &self.environment
    // }

    fn reduce<'a, 'b>(
        context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Action> {
        match action {
            Action::Initial => {
                state.counter = 0;
                return Effect::Action(Action::Load);
            }
            Action::CreateChat => environment.chats.with_mutation(|mut data| {
                let new_chat = Chat {
                    id: (data.len() + 1) as u64,
                    with: format!("id {}", data.len()),
                    messages: vec![crate::model::Message::Send("This is a test".to_string())],
                };
                data.push(new_chat);
            }),
            Action::Load => state.counter += 1,
            Action::Selected(item) => {
                let chat = environment
                    .chats
                    .with(|chats| chats.iter().find(|s| s.id == item).map(|e| e.clone()));
                state.selected = chat;
            }
            Action::Reload => {
                context.send_children(Message::Reload);
            }
            Action::ClosedMessage => {
                state.selected = None;
            }
        }
        Effect::Nothing
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }
}

#[inline_props]
pub fn root<'a>(cx: Scope<'a>, store: VviewStore<'a, RootReducer>) -> Element<'a> {
    println!("re-render root");
    render! {
        div {
            display: "flex",
            flex_direction: "row",
            self::sidebar {
                store: store
            }
            store.selected.as_ref().map(|chat| rsx!(self::selected_message {
                store: store,
                chat: chat
            }))
        }
    }
}

#[inline_props]
fn sidebar<'a>(cx: Scope<'a>, store: &'a VviewStore<'a, RootReducer>) -> Element<'a> {
    render! {
        div {
            button {
                onclick: move |_| store.send(Action::CreateChat),
                "New Chat"
            }
        }
        crate::sidebar::root {
            store: store.host(cx, || crate::sidebar::State::new())
        }
    }
}

#[inline_props]
fn selected_message<'a>(
    cx: Scope<'a>,
    store: &'a VviewStore<'a, RootReducer>,
    chat: &'a Chat,
) -> Element<'a> {
    render! {
        crate::chat::root {
            key: "{chat.id}",
            store: store.host_with(cx, (*chat).clone(), |s| crate::chat::State::new(s)),
        }
    }
}
