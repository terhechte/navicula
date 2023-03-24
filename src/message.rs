use std::rc::Rc;

use crate::{
    model::{Chat, Message},
    navicula::{
        self,
        traits::{Reducer, ReducerContext, VviewStore},
    },
};
use dioxus::prelude::*;

pub struct ChildReducer {
    // environment: super::Environment,
}

pub struct State {
    chat: Chat,
}

impl State {
    pub fn new(chat: Chat) -> Self {
        Self { chat }
    }
}

#[derive(Clone)]
pub enum ChildMessage {}

// impl IntoAction<Action> for Message {
//     fn into_action(self) -> Action {
//         Action::Initial
//     }
// }

#[derive(Clone)]
pub enum DelegateMessage {
    Closed,
}

#[derive(Clone, Debug)]
pub enum Action {
    Initial,
    Close,
}

impl navicula::traits::Reducer for ChildReducer {
    type Message = ChildMessage;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn reduce<'a, 'b>(
        context: &'a ReducerContext<'a, Self::Action, Self::Message, Self::DelegateMessage>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> navicula::Effect<'b, Self::Action> {
        dbg!(&action);
        match action {
            Action::Initial => {}
            Action::Close => {}
        }
        navicula::Effect::Nothing
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }

    // fn environment(&self) -> &Self::Environment {
    //     &self.environment
    // }
}

// Implement the conversion for the `Root` parent
impl navicula::traits::ChildReducer<crate::root::RootReducer> for ChildReducer {
    fn to_child(
        message: <crate::root::RootReducer as Reducer>::Message,
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
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, ChildReducer>) -> Element<'a> {
    println!("re-render child");
    render! {
        for message in store.chat.messages.iter() {
            Message {
                message: &message
            }
        }
    }
}

#[inline_props]
pub fn Message<'a>(cx: Scope<'a>, message: &'a Message) -> Element<'a> {
    render! {
        p {
            match message {
                Message::Send(s) => rsx!("{s}"),
                Message::Received(s) => rsx!("{s}"),
            }
        }
    }
}
