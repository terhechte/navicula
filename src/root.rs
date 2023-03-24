use dioxus::prelude::*;

use crate::navicula::{
    self,
    traits::{IntoAction, VviewStore},
    Effect,
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

impl IntoAction<Action> for Message {
    fn into_action(self) -> Action {
        Action::Message(self)
    }
}

#[derive(Clone)]
pub enum DelegateMessage {
    Closed,
}

#[derive(Clone, Debug)]
pub enum Action {
    Initial,
    Load,
    Message(Message),
}

pub struct State {
    counter: usize,
}

impl State {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl navicula::traits::Reducer for RootReducer {
    type Message = Message;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn initial_state() -> Self::State {
        State { counter: 0 }
    }

    // Provide the environment
    // fn environment(&self) -> &Self::Environment {
    //     &self.environment
    // }

    fn reduce<'a, 'b>(
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Action> {
        match dbg!(action) {
            Action::Initial => {
                state.counter = 0;
                return Effect::Action(Action::Load);
            }
            Action::Load => state.counter += 1,
            Action::Message(m) => match m {
                Message::Reload => state.counter = 0,
            },
        }
        Effect::Nothing
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }
}

#[inline_props]
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, RootReducer>) -> Element<'a> {
    render! {
        div {
            "Root!",
            crate::sidebar::Root {
                store: store.host(cx)
            }
        }
    }
}
