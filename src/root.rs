use dioxus::prelude::*;

use crate::{
    model::Chat,
    navicula::{
        self,
        traits::{ReducerContext, VviewStore},
        Effect,
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
                let chats = environment.chats();
                let chat = chats.iter().find(|s| s.id == item);
                state.selected = chat.cloned();
            }
            Action::Reload => {
                println!("handle reload");
                context.send_children(Message::Reload);
            }
            Action::ClosedMessage => {
                println!("closed chat");
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
pub fn Root<'a>(cx: Scope<'a>, store: VviewStore<'a, RootReducer>) -> Element<'a> {
    println!("re-render root");
    let item = store.selected.as_ref().map(|s| {
        rsx!(crate::message::Root {
            store: store.host(cx, || crate::message::State::new(s.clone())),
        })
    });
    render! {
        div {
            display: "flex",
            flex_direction: "row",
            Sidebar {
                store: store
            }
            SelectedMessage {
                store: store,
            }
            span {
                onclick: move |_| store.send(Action::Reload),
                "Lets talk to our children"
            }
        }
    }
}

#[inline_props]
fn Sidebar<'a>(cx: Scope<'a>, store: &'a VviewStore<'a, RootReducer>) -> Element<'a> {
    render! {
        crate::sidebar::Root {
            store: store.host(cx, || crate::sidebar::State::new())
        }
    }
}

#[inline_props]
fn SelectedMessage<'a>(cx: Scope<'a>, store: &'a VviewStore<'a, RootReducer>) -> Element<'a> {
    render! {
        store.selected.as_ref().map(|s| {
        rsx!(crate::message::Root {
            store: store.host(cx, || crate::message::State::new(s.clone())),
        })
    })
    }
}
