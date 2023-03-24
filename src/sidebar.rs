use std::rc::Rc;

use crate::{
    model::Chat,
    navicula::{
        self,
        traits::{IntoAction, Reducer, VviewStore},
    },
};
use dioxus::prelude::*;

pub struct ChildReducer {
    // environment: super::Environment,
}

pub struct State {
    chats: Rc<Vec<Chat>>,
}

#[derive(Clone)]
pub enum Message {}

impl IntoAction<Action> for Message {
    fn into_action(self) -> Action {
        Action::Initial
    }
}

#[derive(Clone)]
pub enum DelegateMessage {
    Selected(u64),
}

#[derive(Clone)]
pub enum Action {
    Initial,
    Select(u64),
    Reload,
}

impl navicula::traits::Reducer for ChildReducer {
    type Message = Message;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn reduce<'a, 'b>(
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> navicula::Effect<'b, Self::Action> {
        navicula::Effect::Nothing
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }

    fn initial_state(environment: &Self::Environment) -> Self::State {
        State {
            chats: environment.chats(),
        }
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
        match message {
            crate::root::Message::Reload => Some(Action::Reload),
        }
    }

    fn from_child(
        message: <Self as Reducer>::DelegateMessage,
    ) -> Option<<crate::root::RootReducer as Reducer>::Action> {
        match message {
            DelegateMessage::Selected(item) => Some(crate::root::Action::Selected(item)),
        }
    }
}

#[inline_props]
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, ChildReducer>) -> Element<'a> {
    render! {
        "Child!"
        span {
            onclick: move |_| store.send(Action::Select(1)),
            "Select"
        }
    }
}
