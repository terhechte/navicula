use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::RwLock;

use fxhash::FxHashMap;

use super::anyhashable::AnyHashable;
use super::effect::Effect;
use super::types::UpdaterContext;

pub trait Publisher {
    type Data;
    fn with_mutation<T>(&self, action: impl FnOnce(RefMut<Self::Data>) -> T) -> T;
}

pub struct AnySubscription(Box<dyn Subscription>);

impl Subscription for AnySubscription {
    fn id(&self) -> u64 {
        self.0.id()
    }
    fn cancel(&self) {
        self.0.cancel()
    }
}

pub trait Subscription {
    fn id(&self) -> u64;
    fn cancel(&self);
}

#[derive(Clone)]
pub struct RefSubscription<Data> {
    id: u64,
    #[allow(clippy::type_complexity)]
    subscribers: Rc<RwLock<FxHashMap<u64, Box<dyn Fn(Rc<RefCell<Data>>) + Send + Sync>>>>,
}

impl<Data: 'static> RefSubscription<Data> {
    fn any_subscription(self) -> AnySubscription {
        AnySubscription(Box::new(self))
    }
}

impl<Data> Subscription for RefSubscription<Data> {
    fn id(&self) -> u64 {
        self.id
    }
    fn cancel(&self) {
        let Ok(mut subscribers) = self.subscribers.write() else {
            log::error!("Could not unsubscribe");
            return
        };
        log::trace!("Cancel Subscription");
        subscribers.remove(&self.id);
    }
}

#[derive(Clone, Default)]
pub struct RefPublisher<Data> {
    data: Rc<RefCell<Data>>,
    version: Rc<Cell<u64>>,
    /// subscribe to storage changes. We're not using a `RefCell` as that can lead
    /// to multiple borrow crashes here
    #[allow(clippy::type_complexity)]
    subscribers: Rc<RwLock<FxHashMap<u64, Box<dyn Fn(Rc<RefCell<Data>>) + Send + Sync>>>>,
}

impl<Data> RefPublisher<Data> {
    pub fn new(data: Data) -> Self {
        Self {
            data: Rc::new(RefCell::new(data)),
            version: Default::default(),
            subscribers: Default::default(),
        }
    }
}

impl<Data: 'static> RefPublisher<Data> {
    pub fn with_mutation<T>(&self, action: impl FnOnce(RefMut<Data>) -> T) -> T {
        let t = {
            let r = self.data.borrow_mut();
            action(r)
        };
        let mut vx = (*self.version).get();

        vx = vx.wrapping_add(1);
        (*self.version).replace(vx);
        {
            let Ok(subscribers) = self.subscribers.read() else {
                log::error!("Poisoned Read lock");
                panic!()
            };
            for (_, updater) in subscribers.iter() {
                (*updater)(self.data.clone());
            }
        }
        t
    }

    pub fn with<T>(&self, action: impl FnOnce(Ref<Data>) -> T) -> T {
        let r = self.data.borrow();
        action(r)
    }

    pub fn subscribe<'x, A: Clone + Send + Sync + 'static, F>(
        &self,
        id: impl Into<AnyHashable>,
        context: &impl UpdaterContext<A>,
        action: F,
    ) -> Effect<'x, A>
    where
        // U: UpdaterContext<A>,
        F: for<'a> Fn(Ref<'a, Data>) -> A + Send + Sync + 'static,
    {
        let update = context.updater().clone();
        let boxed_updater = Box::new(move |data: Rc<RefCell<Data>>| {
            let Ok(reference) = data.try_borrow() else {
                log::error!("Could not borrow data");
                return
            };
            let a = action(reference);
            update(a);
        });

        // first call
        boxed_updater(self.data.clone());

        let id = id.into().id();
        {
            let mut subscribers = self.subscribers.write().unwrap();
            subscribers.insert(id, boxed_updater);
        }
        Effect::subscription(
            RefSubscription {
                id,
                subscribers: self.subscribers.clone(),
            }
            .any_subscription(),
        )
    }

    pub fn unsubscribe(&self, id: impl Into<AnyHashable>) {
        let id = id.into().id();
        let Ok(mut subscribers) = self.subscribers.write() else {
            log::error!("Could not unsubscribe");
            return
        };
        subscribers.remove(&id);
    }
}
