#![allow(unused)]
use std::sync::Arc;

use dioxus::prelude::Scope;
use dioxus_desktop::{use_window, DesktopContext, EvalResult};

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

pub trait UpdaterContext<Action> {
    fn updater(&self) -> &Arc<dyn Fn(Action) + Send + Sync>;
}

pub trait MessageContext<Action, DelegateMessage, Message>: UpdaterContext<Action> {
    fn send_parent(&self, message: DelegateMessage);
    fn send_children(&self, message: Message);
    fn window(&self) -> &AppWindow;
}
