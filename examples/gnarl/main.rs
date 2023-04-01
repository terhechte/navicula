pub mod chat;
pub mod model;
pub mod root;
pub mod sidebar;

use dioxus::prelude::*;

use dioxus_desktop::Config;

fn main() {
    use env_logger::Env;
    use std::io::Write;
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("trace"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                //chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .target(env_logger::Target::Stdout)
        .init();

    let config = Config::default();
    dioxus_desktop::launch_with_props(self::app, AppProps {}, config);
}

pub struct AppProps {}

pub fn app<'a>(cx: Scope<'a, AppProps>) -> Element<'a> {
    let environment = use_state(cx, || model::Environment::new(model::mock::chats()));
    let store = navicula::logic::root(cx, &[], environment.get(), || root::State::new());
    render! {
        root::root {
            store: store
        }
    }
}
