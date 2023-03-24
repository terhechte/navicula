use crate::navicula::{
    self,
    traits::{IntoAction, Reducer, VviewStore},
};
use dioxus::prelude::*;

pub struct ChildReducer {
    // environment: super::Environment,
}

impl ChildReducer {
    pub fn new() -> Self {
        ChildReducer {}
    }
}

pub struct State;

#[derive(Clone)]
pub enum Message {}

impl IntoAction<Action> for Message {
    fn into_action(self) -> Action {
        Action::Initial
    }
}

#[derive(Clone)]
pub enum DelegateMessage {}

#[derive(Clone)]
pub enum Action {
    Initial,
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

    fn initial_state() -> Self::State {
        State
    }

    // fn environment(&self) -> &Self::Environment {
    //     &self.environment
    // }
}

impl<Parent: Reducer> navicula::traits::ChildReducer<Parent> for ChildReducer {
    fn to_child(message: <Parent as Reducer>::Message) -> Option<<Self as Reducer>::Action> {
        panic!()
    }

    fn from_child(message: <Self as Reducer>::DelegateMessage) -> Option<Parent::Action> {
        todo!()
    }
}

#[inline_props]
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, ChildReducer>) -> Element<'a> {
    render! {
        "Child!"
    }
}
