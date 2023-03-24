mod navicula;

// Temporary!
pub mod message;
pub mod model;
pub mod root;
pub mod sidebar;

use dioxus::prelude::*;

use dioxus_desktop::Config;

fn main() {
    let config = Config::default();
    dioxus_desktop::launch_with_props(App, AppProps {}, config);
}

pub struct AppProps {}

pub fn App<'a>(cx: Scope<'a, AppProps>) -> Element<'a> {
    let environment = use_state(cx, || model::Environment::new(model::mock::chats()));
    let store = navicula::traits::root(cx, environment.get());
    render! {
        root::Root {
            store: store
        }
    }
}
