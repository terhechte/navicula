mod navicula;
use dioxus::prelude::*;

use dioxus_desktop::Config;

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
