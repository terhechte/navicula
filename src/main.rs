mod navicula;
use dioxus::prelude::*;

use dioxus_desktop::Config;
use navicula::traits::EnvironmentType;

fn main() {
    let config = Config::default();
    dioxus_desktop::launch_with_props(RootApp, RootAppProps {}, config);
}

pub struct RootAppProps {}

pub fn RootApp<'a>(cx: Scope<'a, RootAppProps>) -> Element<'a> {
    render! {
        div {
            "Hello"
        }
    }
}

mod root {
    use crate::navicula::{self, traits::IntoAction, Effect};

    pub struct RootReducer {
        environment: super::Environment,
    }

    impl Default for RootReducer {
        fn default() -> Self {
            Self {
                environment: Default::default(),
            }
        }
    }

    #[derive(Clone)]
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

    #[derive(Clone)]
    pub enum Action {
        Initial,
        Load,
        Message(Message),
    }

    pub struct State {
        counter: usize,
    }

    impl navicula::traits::Reducer for RootReducer {
        type Message = Message;

        type DelegateMessage = DelegateMessage;

        type Action = Action;

        type State = State;

        type Environment = super::Environment;

        fn initial_state() -> Self::State {
            State { counter: 0 }
        }

        /// Provide the environment
        fn environment(&self) -> &Self::Environment {
            &self.environment
        }

        fn reduce<'a, 'b>(
            action: Self::Action,
            state: &'a mut Self::State,
            environment: &'a Self::Environment,
        ) -> Effect<'b, Action> {
            match action {
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
}

#[derive(Clone, Default)]
pub struct Environment {}

impl Environment {
    pub fn party(&self) {}
}

impl EnvironmentType for Environment {
    type AppEvent = usize;
}
