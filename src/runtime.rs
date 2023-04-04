use std::rc::Rc;
use std::sync::RwLock;

use super::publisher::AnySubscription;
use super::reducer::Reducer;
use super::types::ActionSender;

pub(crate) struct Runtime<R: Reducer> {
    pub(crate) scope_id: usize,
    /// Send an action that will be processed afterwards
    pub(crate) sender: ActionSender<R::Action>,
    /// The current registered children. Used to send them messages
    pub(crate) child_senders: RwLock<fxhash::FxHashMap<usize, Rc<dyn Fn(R::Message)>>>,
    /// Current subscriptions so they can be cleared on drop
    pub(crate) subscriptions: RwLock<Vec<AnySubscription>>,
}

impl<R: Reducer> Runtime<R> {
    pub(crate) fn new(scope_id: usize, sender: ActionSender<R::Action>) -> Self {
        Self {
            scope_id,
            sender,
            child_senders: Default::default(),
            subscriptions: Default::default(),
        }
    }
}

impl<R: Reducer> std::fmt::Debug for Runtime<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rruntime")
            .field("scope_id", &self.scope_id)
            .field("subscriptions", &self.subscriptions.read().unwrap().len())
            .finish()
    }
}
