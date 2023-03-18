use dioxus::core::Scope;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;

#[derive(Clone)]
pub struct Notifier<Action: Clone + 'static> {
    sender: Rc<Mutex<Option<Box<dyn Fn(Action)>>>>,
}

impl<Action: Clone> Notifier<Action> {
    pub fn new(sender: Box<dyn Fn(Action)>) -> Self {
        Self {
            sender: Rc::new(Mutex::new(Some(sender))),
        }
    }

    pub fn execute(&self, action: Action) {
        let Ok(mut lock) = self.sender.lock() else { return };
        let Some(f) = lock.take() else { return };
        f(action);
        lock.replace(f);
    }
}

impl<Action: Clone> std::fmt::Debug for Notifier<Action> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notifier").finish()
    }
}

pub trait ScopeNotifyExt {
    fn waker<Action: Clone + 'static>(&self) -> Notifier<Action>;
}

impl<'a, T> ScopeNotifyExt for Scope<'a, T> {
    fn waker<Action: Clone + 'static>(&self) -> Notifier<Action> {
        let sender = self
            .consume_context::<Waker<Action>>()
            .expect("Expect to have a registered waker");

        let waker = self.schedule_update();
        Notifier::new(Box::new(move |action| {
            if let Err(e) = sender.0.send(action) {
                log::error!("Could not send wake: {e:?}");
            }
            waker();
        }))
    }
}

#[derive(Clone)]
struct Waker<Action: Clone>(Sender<Action>);

pub(super) fn register_or_insert_waker<'a, T, Action: Clone + 'static>(
    cx: Scope<'a, T>,
) -> Receiver<Action> {
    let (sender, receiver) = std::sync::mpsc::channel::<Action>();
    cx.provide_context(Waker(sender));
    receiver
}

#[cfg(target_arch = "wasm32")]
pub fn mock_waker<Action: std::fmt::Debug + Clone>() -> Notifier<Action> {
    Notifier::new(Box::new(move |action| {
        log::debug!("Notify {action:?}");
    }))
}
