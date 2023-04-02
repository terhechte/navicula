#![allow(unused)]
use std::sync::Arc;

use dioxus::prelude::Scope;
use dioxus_desktop::{use_window, DesktopContext, EvalResult};

#[derive(Copy, Clone)]
pub struct AppWindow<'a> {
    window: &'a DesktopContext,
}

impl<'a> AppWindow<'a> {
    pub fn retrieve<T>(cx: Scope<'a, T>) -> AppWindow<'a> {
        AppWindow {
            window: use_window(cx),
        }
    }

    pub fn eval(&self, code: &str) -> EvalResult {
        self.window.eval(code)
    }
}

impl<'a> std::ops::Deref for AppWindow<'a> {
    type Target = DesktopContext;

    fn deref(&self) -> &Self::Target {
        self.window
    }
}

pub trait UpdaterContext<Action> {
    fn window(&self) -> &AppWindow;
    fn updater(&self) -> &Arc<dyn Fn(Action) + Send + Sync>;
    fn render(&self);
}

pub trait MessageContext<Action, DelegateMessage, Message>: UpdaterContext<Action> {
    fn send_parent(&self, message: DelegateMessage);
    fn send_children(&self, message: Message);
}

#[derive(Clone)]
pub struct ActionSender<Action: Clone> {
    pub(crate) sender: flume::Sender<Action>,
    pub(crate) updater: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl<Action: Clone> ActionSender<Action> {
    pub fn send(&self, action: Action) {
        if let Err(e) = self.sender.send(action) {
            log::error!("Could not send action {e:?}");
        }
        (*self.updater)();
    }
}

pub trait EnvironmentType {
    type AppEvent;
}
