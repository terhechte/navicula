// use crate::{
//     environment::{types::ActionFromEvent, types::MainMenuEvent},
//     helper::supported_file_types,
//     reducer::Action,
// };
use flume::{Receiver, Sender};
use std::path::PathBuf;
use std::sync::Arc;

// use crate::environment::model::Message;

pub trait EventReceiver<T: Clone> {
    fn receive(&self) -> Option<T>;
}

#[derive(Clone)]
pub struct TimerEventReceiver<A> {
    receiver: Receiver<A>,
}

impl<A> TimerEventReceiver<A> {
    pub fn new(receiver: Receiver<A>) -> Self {
        Self { receiver }
    }
}

impl<A: Clone> EventReceiver<A> for TimerEventReceiver<A> {
    fn receive(&self) -> Option<A> {
        self.receiver.try_recv().ok()
    }
}

#[derive(Clone)]
pub struct ActionEventReceiver<A> {
    receiver: Receiver<A>,
}

impl<A> ActionEventReceiver<A> {
    pub fn new(receiver: Receiver<A>) -> Self {
        Self { receiver }
    }
}

impl<A: Clone> EventReceiver<A> for ActionEventReceiver<A> {
    fn receive(&self) -> Option<A> {
        self.receiver.try_recv().ok()
    }
}

/*
#[derive(Clone)]
pub struct StreamEventReceiver {
    receiver: Receiver<Message>,
}

impl StreamEventReceiver {
    pub fn channel(
        waker: Arc<dyn Fn() + Send + Sync>,
    ) -> (Arc<dyn Fn(Message) + Send + Sync>, Self) {
        let (sender, receiver) = flume::unbounded();
        (
            Arc::new(move |msg| {
                if let Err(e) = sender.send(msg) {
                    log::error!("Could not send msg: {e:?}");
                }
                waker();
            }),
            StreamEventReceiver { receiver },
        )
    }
}

impl EventReceiver<Action> for StreamEventReceiver {
    fn receive(&self) -> Option<Action> {
        self.receiver.try_recv().ok().map(Action::MessageEvent)
    }
}*/

#[derive(Clone, Debug)]
pub enum AppEvent {
    FocusChange(bool),
    // FIXME: Find an abstraction where MenuEvent can be generic

    //MenuEvent(crate::environment::types::MainMenuEvent),
    FileEvent(FileEvent),
    ClosingWindow,
}

/*
impl ActionFromEvent for AppEvent {
    fn make_focus_event(focus: bool) -> Option<Self>
    where
        Self: Sized,
    {
        Some(AppEvent::FocusChange(focus))
    }
    fn make_menu_event(event: MainMenuEvent) -> Option<Self>
    where
        Self: Sized,
    {
        Some(AppEvent::MenuEvent(event))
    }
    fn make_close_window_event() -> Option<Self>
    where
        Self: Sized,
    {
        Some(AppEvent::ClosingWindow)
    }
}
*/

#[derive(Clone, Debug)]
pub enum FileEvent {
    Hovering(bool),
    Dropped(Vec<PathBuf>),
    Cancelled,
}

#[derive(Clone)]
pub struct AppEventReceiver<T: Clone> {
    receiver: Receiver<AppEvent>,
    mapper: fn(AppEvent) -> T,
}

pub type DefaultEventReceiver<T = AppEvent> = AppEventReceiver<T>;

impl<T: Clone> std::fmt::Debug for AppEventReceiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FileEventReceiver").finish()
    }
}

impl AppEventReceiver<AppEvent> {
    pub fn new(receiver: Receiver<AppEvent>) -> Self {
        Self {
            receiver,
            mapper: Self::identity,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn mock() -> (Sender<AppEvent>, Self) {
        let (sender, receiver) = flume::unbounded();

        (
            sender,
            Self {
                receiver,
                mapper: Self::identity,
            },
        )
    }

    pub fn map<Action: Clone>(self, mapper: fn(AppEvent) -> Action) -> AppEventReceiver<Action> {
        AppEventReceiver {
            receiver: self.receiver,
            mapper,
        }
    }

    fn identity(m: AppEvent) -> AppEvent {
        m
    }
}

impl<T: Clone> EventReceiver<T> for AppEventReceiver<T> {
    fn receive(&self) -> Option<T> {
        self.receiver.try_recv().ok().map(|e| (self.mapper)(e))
    }
}

#[cfg(not(target_arch = "wasm32"))]
use dioxus_desktop::{tao::window::Window, wry::webview::FileDropEvent};

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_file_event(
    _window: &Window,
    file: FileDropEvent,
    updater: Arc<dyn Fn() + Send + Sync>,
    sender: Sender<AppEvent>,
) -> bool {
    match file {
        FileDropEvent::Hovered(files) => {
            let allowed = !files.is_empty();
            if let Err(e) = sender.send(AppEvent::FileEvent(FileEvent::Hovering(allowed))) {
                log::error!("drag drop: {e:?}");
            }
            updater()
        }
        FileDropEvent::Dropped(files) => {
            if !files.is_empty() {
                if let Err(e) = sender.send(AppEvent::FileEvent(FileEvent::Dropped(files))) {
                    log::error!("drag drop: {e:?}");
                }
                updater()
            }
        }
        FileDropEvent::Cancelled => {
            if let Err(e) = sender.send(AppEvent::FileEvent(FileEvent::Cancelled)) {
                log::error!("drag drop: {e:?}");
            }
            updater()
        }
        _ => (),
    }
    true
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_app_event_channel() -> (DefaultEventReceiver, Sender<AppEvent>) {
    let (sender, receiver) = flume::unbounded();
    let wrapped = AppEventReceiver::new(receiver);
    (wrapped, sender)
}
