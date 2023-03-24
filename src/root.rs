use dioxus::prelude::*;

use crate::navicula::{
    self,
    traits::{ReducerContext, VviewStore},
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
    Selected(u64),
}

pub struct State {
    counter: usize,
    selected: Option<u64>,
}

impl navicula::traits::Reducer for RootReducer {
    type Message = Message;

    type DelegateMessage = DelegateMessage;

    type Action = Action;

    type State = State;

    type Environment = crate::model::Environment;

    fn initial_state(environment: &Self::Environment) -> Self::State {
        State {
            counter: 0,
            selected: None,
        }
    }

    // Provide the environment
    // fn environment(&self) -> &Self::Environment {
    //     &self.environment
    // }

    fn reduce<'a, 'b>(
        context: &'a ReducerContext<'a, Self::Action, Self::Message, Self::DelegateMessage>,
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
            Action::Selected(item) => {
                state.selected = Some(item);
            }
            Action::Reload => {
                println!("handle reload");
                context.send_children(Message::Reload);
            }
        }
        Effect::Nothing
    }

    fn initial_action() -> Option<Self::Action> {
        Some(Action::Initial)
    }
}

#[inline_props]
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, RootReducer>) -> Element<'a> {
    println!("re-render root");
    let item = store.selected.map(|s| rsx!("Selected {s}"));
    render! {
        div {
            "Root!",
            crate::sidebar::Root {
                store: store.host(cx)
            }
            item
            span {
                onclick: move |_| store.send(Action::Reload),
                "Lets talk to our children"
            }
        }
    }
}
