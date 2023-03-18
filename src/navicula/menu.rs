use dioxus::prelude::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use std::any::Any;

use crate::environment::platform::AppWindow;

type MenuEventId = u32;

lazy_static::lazy_static! {
    static ref CONTEXT_MENU_STATE: Arc<RwLock<Option<HashMap<MenuEventId, Box<dyn Any + Send + Sync>>>>> = Arc::default();
    static ref CONTEXT_MENURESULT: Arc<RwLock<Option<Box<dyn Any + Send + Sync>>>> = Arc::default();
}

pub trait ScopeExt {
    fn window(&self) -> &AppWindow;
}

type Payload = Box<dyn Any + Send + Sync>;

#[allow(unused)]
enum ContextMenuKind {
    Checkbox {
        title: String,
        checked: bool,
        payload: Payload,
    },
    Item {
        title: String,
        payload: Payload,
    },
    Submenu {
        title: String,
        children: Vec<ContextMenuItem>,
    },
    Separator,
}

pub struct ContextMenuItem {
    // Only one field to hide the actual enum
    kind: ContextMenuKind,
}

pub struct ContextMenu {
    title: String,
    enabled: bool,
    children: Vec<ContextMenuItem>,
}

impl ContextMenu {
    pub fn new(title: impl AsRef<str>, enabled: bool, children: Vec<ContextMenuItem>) -> Self {
        Self {
            title: title.as_ref().to_string(),
            enabled,
            children,
        }
    }
}

#[allow(unused)]
impl ContextMenuItem {
    pub fn item<T: Send + Sync + 'static>(title: impl AsRef<str>, payload: T) -> Self {
        Self {
            kind: ContextMenuKind::Item {
                title: title.as_ref().to_string(),
                payload: Box::new(payload),
            },
        }
    }
    pub fn checkbox<T: Send + Sync + 'static>(
        title: impl AsRef<str>,
        checked: bool,
        payload: T,
    ) -> Self {
        Self {
            kind: ContextMenuKind::Checkbox {
                title: title.as_ref().to_string(),
                checked,
                payload: Box::new(payload),
            },
        }
    }
    pub fn submenu(title: impl AsRef<str>, children: Vec<ContextMenuItem>) -> Self {
        Self {
            kind: ContextMenuKind::Submenu {
                title: title.as_ref().to_string(),
                children,
            },
        }
    }

    pub fn separator() -> Self {
        Self {
            kind: ContextMenuKind::Separator,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ContextMenuItem {
    fn build(self, into: &mut muda::Submenu, actions: &mut HashMap<MenuEventId, Payload>) {
        use muda::{CheckMenuItem, MenuItem, PredefinedMenuItem, Submenu};
        match self.kind {
            ContextMenuKind::Checkbox {
                title,
                checked,
                payload,
            } => {
                let item = CheckMenuItem::new(title, true, checked, None);
                actions.insert(item.id(), payload);
                into.append(&item);
            }
            ContextMenuKind::Item { title, payload } => {
                let item = MenuItem::new(title, true, None);
                actions.insert(item.id(), payload);
                into.append(&item);
            }
            ContextMenuKind::Submenu { title, children } => {
                let mut sub_menu = Submenu::new(title, true);
                for child in children {
                    child.build(&mut sub_menu, actions);
                }
                into.append(&sub_menu);
            }
            ContextMenuKind::Separator => {
                into.append(&PredefinedMenuItem::separator());
            }
        }
    }
}

impl<'a, T> ScopeExt for &'a Scoped<'a, T> {
    fn window(&self) -> &'a AppWindow {
        crate::environment::platform::window(self)
    }
}

pub fn context_menu(window: &AppWindow, event: &MouseData, menu: ContextMenu) {
    let _pos = event.client_coordinates();

    #[cfg(not(target_arch = "wasm32"))]
    {
        use muda::{ContextMenu, Submenu};

        let mut context_menu = Submenu::new(menu.title, menu.enabled);
        let mut actions = HashMap::new();
        for child in menu.children {
            child.build(&mut context_menu, &mut actions);
        }

        if let Ok(mut t) = CONTEXT_MENU_STATE.write() {
            t.replace(actions);
        }

        #[cfg(target_os = "macos")]
        {
            use dioxus_desktop::wry::webview::WebviewExtMacOS;
            use objc::runtime::Object;
            let scale_factor = window.scale_factor();
            let vx = window.webview.webview();
            // let ux = v.inner_size().height as f64;

            let xp = unsafe {
                // use cacao::appkit::App;
                use cocoa::appkit::NSApp;
                // use cacao::core_graphics::NSP
                use cocoa::foundation::NSPoint;
                let app = NSApp();
                let o: *mut Object = msg_send![app, currentEvent];
                let mut p: NSPoint = msg_send![o, locationInWindow];
                p.x += 5.;
                p.y -= 12.;
                p
            };

            context_menu.show_context_menu_for_nsview(
                vx as _,
                // pos.x * scale_factor,
                // ux - (pos.y * scale_factor),
                xp.x * scale_factor,
                xp.y * scale_factor,
            )
        }
    }
}
// }

pub(super) fn setup_menu_handler(schedule_update: Arc<dyn Fn() + Send + Sync>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use muda::MenuEvent;
        MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            if let Some(Some(mut actions)) = CONTEXT_MENU_STATE.write().ok().map(|mut e| e.take()) {
                if let Some(action) = actions.remove(&event.id) {
                    if let Err(e) = CONTEXT_MENURESULT.write().map(|mut e| e.replace(action)) {
                        log::error!("Could not set action: {e:?}");
                    }
                }
            }
            schedule_update()
        }));
    }
}

pub(super) fn resolve_current_action<Action: Clone + 'static>() -> Option<Action> {
    let value = {
        let context_result = CONTEXT_MENURESULT.try_read().ok();
        if let Some(Some(e)) = context_result.as_deref() {
            if let Some(value) = e.downcast_ref::<Action>() {
                // We have to clone here. Otherwise the read access to CONTEXT_MENURESULT survives
                // past this block and we can't clear it below.
                let cloned = value.clone();
                Some(cloned)
            } else {
                None
            }
        } else {
            None
        }
    };

    if value.is_some() {
        if let Err(e) = CONTEXT_MENURESULT.try_write().map(|mut e| *e = None) {
            log::error!("Could not clear context: {e:?}");
        }
    }
    value
}
