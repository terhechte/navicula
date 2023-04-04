use dioxus::prelude::*;
use navicula::{
    self, effect::Effect, reducer::Reducer, types::MessageContext, viewstore::ViewStore,
};

use crate::model::Chat;

pub struct RootReducer {}

#[derive(Clone, Debug)]
pub enum Message {
    Reload,
}

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

impl Reducer for RootReducer {
    type Message = Message;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn reduce<'a, 'b>(
        context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Action> {
        match action {
            Action::Initial => {
                state.counter = 0;
                return Effect::action(Action::Load);
            }
            Action::CreateChat => environment.chats.with_mutation(|mut data| {
                let new_chat = Chat {
                    id: (data.len() + 1) as u64,
                    with: format!("id {}", data.len()),
                    messages: vec![crate::model::Message::Send("This is a test".to_string())],
                };
                data.push(new_chat);
            }),
            Action::Load => {
                state.counter += 1;
            }
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
        Effect::NONE
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }

    fn register_sideeffects(_sender: &navicula::types::ActionSender<Self::Action>) {}
}

#[inline_props]
pub fn root<'a>(cx: Scope<'a>, store: ViewStore<'a, RootReducer>) -> Element<'a> {
    println!("root {:?}", cx.scope_id());
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
fn sidebar<'a>(cx: Scope<'a>, store: &'a ViewStore<'a, RootReducer>) -> Element<'a> {
    println!("sidebar {:?}", cx.scope_id());
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
    store: &'a ViewStore<'a, RootReducer>,
    chat: &'a Chat,
) -> Element<'a> {
    cx.use_hook(|| Drops(cx.scope_id().0));
    println!("selected_message {:?}", cx.scope_id());
    render! {
        crate::chat::root {
            key: "{chat.id}",
            store: store.host_with(cx, *chat, |s| crate::chat::State::new(s)),
        }
    }
}

struct Drops(usize);

impl Drop for Drops {
    fn drop(&mut self) {
        println!("Dropped Selected Message!: {}", self.0);
    }
}
