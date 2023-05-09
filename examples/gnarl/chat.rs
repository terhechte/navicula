use navicula::{
    self,
    effect::Effect,
    reducer::{ChildReducer, Reducer},
    types::MessageContext,
    viewstore::ViewStore,
};
use std::time::Duration;

use crate::model::{Chat, Message};
use dioxus::prelude::*;

pub struct ChatChildReducer {
    // environment: super::Environment,
}

pub struct State {
    chat: Chat,
}

impl State {
    pub fn new(chat: Chat) -> Self {
        Self { chat: chat.clone() }
    }
}

#[derive(Clone)]
pub enum ChildMessage {}

#[derive(Clone)]
pub enum DelegateMessage {
    Closed,
}

#[derive(Clone, Debug)]
pub enum Action {
    Initial,
    Close,
    Chats(usize),
    Edit(Message),
    Edit2(Message),
    FinishEdit,
}

impl Reducer for ChatChildReducer {
    type Message = ChildMessage;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn reduce<'a, 'b>(
        context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
        action: Self::Action,
        _state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Self::Action> {
        match action {
            Action::Initial => {
                // fake subscription, just to see if drop works
                return environment
                    .chats
                    .subscribe("chat-chats", context, |data| Action::Chats(data.len()));
            }
            Action::Chats(cnt) => {
                log::info!("Have {cnt} chats");
            }
            Action::Edit(message) => {
                return Effect::action(Action::Edit2(message)).delay(Duration::from_secs(2))
            }
            Action::Edit2(message) => {
                environment
                    .selected
                    .with_mutation(|mut s| *s = Some(message));
            }
            Action::FinishEdit => {
                environment.selected.with_mutation(|mut s| *s = None);
            }
            Action::Close => {
                context.send_parent(DelegateMessage::Closed);
            }
        }
        Effect::NONE
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }
}

// Implement the conversion for the `Root` parent
impl ChildReducer<crate::root::RootReducer> for ChatChildReducer {
    fn to_child(
        _message: <crate::root::RootReducer as Reducer>::Message,
    ) -> Option<<Self as Reducer>::Action> {
        None
    }

    fn from_child(
        message: <Self as Reducer>::DelegateMessage,
    ) -> Option<<crate::root::RootReducer as Reducer>::Action> {
        match message {
            DelegateMessage::Closed => Some(crate::root::Action::ClosedMessage),
        }
    }
}

#[inline_props]
pub fn root<'a>(cx: Scope<'a>, store: ViewStore<'a, ChatChildReducer>) -> Element<'a> {
    render! {
        div {
            display: "flex",
            flex_direction: "column",
            span {
                onclick: move |_| store.send(Action::Close),
                "CLOSE"
            }
            for message in store.chat.messages.iter() {
                self::message {
                    message: &message,
                    store: store
                }
            }
            hr {}
            edit::root {
                store: store.host(cx, || Default::default())
            }
        }
    }
}

#[inline_props]
pub fn message<'a>(
    cx: Scope<'a>,
    message: &'a Message,
    store: &'a ViewStore<'a, ChatChildReducer>,
) -> Element<'a> {
    render! {
        p {
            a {
                onclick: move |_| store.send(Action::Edit((*message).clone())),
                "Edit"
            }
            match message {
                Message::Send(s) => rsx!("{s}"),
                Message::Received(s) => rsx!("{s}"),
            }
        }
    }
}

mod edit {
    use super::*;
    pub struct EditReducer;

    #[derive(Default)]
    pub struct EditState {
        pub message: Option<Message>,
    }

    #[derive(Clone)]
    pub enum ChildMessage {}

    #[derive(Clone)]
    pub enum DelegateMessage {
        Done,
    }

    #[derive(Clone, Debug)]
    pub enum EditAction {
        Initial,
        ReceivedMessage(Option<Message>),
        Done,
    }

    impl Reducer for EditReducer {
        type Message = ChildMessage;
        type DelegateMessage = DelegateMessage;
        type Action = EditAction;
        type State = EditState;
        type Environment = crate::model::Environment;

        fn reduce<'a, 'b>(
            context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
            action: Self::Action,
            state: &'a mut Self::State,
            environment: &'a Self::Environment,
        ) -> Effect<'b, Self::Action> {
            match action {
                EditAction::Initial => {
                    return environment
                        .selected
                        .subscribe("selected", context, |message| {
                            let m = message.clone();
                            EditAction::ReceivedMessage(m)
                        });
                }
                EditAction::ReceivedMessage(message) => {
                    state.message = message;
                }
                EditAction::Done => context.send_parent(DelegateMessage::Done),
            }
            Effect::NONE
        }

        fn initial_action() -> Option<Self::Action> {
            Some(EditAction::Initial)
        }
    }

    impl ChildReducer<ChatChildReducer> for EditReducer {
        fn to_child(
            _message: <ChatChildReducer as Reducer>::Message,
        ) -> Option<<Self as Reducer>::Action> {
            None
        }

        fn from_child(
            message: <Self as Reducer>::DelegateMessage,
        ) -> Option<<ChatChildReducer as Reducer>::Action> {
            match message {
                DelegateMessage::Done => Some(<ChatChildReducer as Reducer>::Action::FinishEdit),
            }
        }
    }

    #[inline_props]
    pub fn root<'a>(cx: Scope<'a>, store: ViewStore<'a, EditReducer>) -> Element<'a> {
        let Some(message) = store.message.as_ref() else {
            return render!(div{})
        };
        let content = match message {
            Message::Received(ref m) => m,
            Message::Send(ref m) => m,
        };
        render! {
            div {
                display: "flex",
                flex_direction: "column",
                "Edit: "
                a {
                    onclick: move |_| store.send(EditAction::Done),
                    "CLOSE"
                }
                textarea {
                    onchange: move |v| println!("{}", v.value),
                    "{content}"
                }
            }
        }
    }
}
