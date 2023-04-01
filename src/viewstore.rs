use super::reducer::Reducer;
use super::runtime::Runtime;
use super::types::ActionSender;
use dioxus::prelude::UseState;

pub struct ViewStore<'a, R: Reducer + 'static> {
    pub(crate) state: &'a R::State,
    pub(crate) environment: &'a R::Environment,
    /// This state is kept separate so that it can be
    /// kept in a `UseState` and is only created once
    pub(crate) runtime: &'a UseState<Runtime<R>>,
}

impl<'a, R: Reducer> Clone for ViewStore<'a, R> {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            environment: self.environment,
            runtime: self.runtime,
        }
    }
}

impl<'a, R: Reducer> ViewStore<'a, R> {
    pub fn send(&self, action: R::Action) {
        self.runtime.get().sender.send(action);
    }

    pub fn sender(&self) -> ActionSender<R::Action> {
        //self.runtime.read().sender.clone()
        self.runtime.get().sender.clone()
    }
}

impl<'a, R: Reducer> std::ops::Deref for ViewStore<'a, R> {
    type Target = R::State;

    fn deref(&self) -> &Self::Target {
        self.state.deref()
    }
}
