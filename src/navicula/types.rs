use dioxus::prelude::Scope;
use dioxus_desktop::{use_window, DesktopContext, EvalResult};

pub struct AppWindow<'a> {
    window: &'a dioxus_desktop::DesktopContext,
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
