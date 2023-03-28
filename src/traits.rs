use dioxus::prelude::*;
use futures_util::{future::BoxFuture, StreamExt};
use fxhash::FxHashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::sync::RwLock;
use std::{rc::Rc, sync::Arc};

use super::publisher::{AnySubscription, Subscription};
use super::types::{MessageContext, UpdaterContext};
use super::{
    effect::{Effect, InnerEffect},
    types::AppWindow,
};

struct Drops(Box<dyn Fn()>);

impl Drops {
    fn register<'a, T>(cx: Scope<'a, T>, msg: &'static str) {
        let id = cx.scope_id().0;
        cx.use_hook(move || {
            Drops(Box::new(move || {
                println!("Dropped: {} {}", id, msg);
            }))
        });
    }

    fn action<'a, T>(cx: Scope<'a, T>, a: impl Fn() + 'static) {
        let boxed = Box::new(move || a());
        cx.use_hook(move || Drops(boxed));
    }
}

impl Drop for Drops {
    fn drop(&mut self) {
        (self.0)()
    }
}

pub trait Reducer {
    /// A reducer can be messaged from a parent.
    /// `Messages` are send parent to child
    type Message: Clone; // + IntoAction<Self::Action>;

    /// This type is used to delegate from a Child Reducer
    /// back to its parent. The parent can then decide whether
    /// to consume this event or whether to ignore it.
    /// DelegateMessages are send child to parent
    type DelegateMessage: Clone;

    /// The action is the internal type of the Reducer. It cannot
    /// be called or accessed by the outside
    type Action: Clone + Send;

    /// The state that this reducer can act upon
    type State;

    // The environment type we're using
    type Environment: EnvironmentType;

    fn reduce<'a, 'b>(
        //context: &'a ReducerContext<'a, Self::Action, Self::Message, Self::DelegateMessage>,
        context: &'a impl MessageContext<Self::Action, Self::DelegateMessage, Self::Message>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Self::Action>;

    /// Define the initial action when the reducer starts up
    fn initial_action() -> Option<Self::Action>;

    // Provide the initial state
    // fn initial_state(environment: &Self::Environment) -> Self::State;

    // Provide the environment
    // fn environment(&self) -> &Self::Environment;
}

pub trait ChildReducer<Parent: Reducer>: Reducer {
    // type Parent: Reducer;

    fn to_child(message: <Parent as Reducer>::Message) -> Option<<Self as Reducer>::Action>;

    fn from_child(message: <Self as Reducer>::DelegateMessage) -> Option<Parent::Action>;
}

// pub trait IntoAction<Action> {
//     fn into_action(self) -> Action;
// }

pub trait EnvironmentType {
    type AppEvent;
}

/// Allows converting any kind of external event system
pub trait IntoMessageSender<Message> {
    fn into_sender(
        &self,
        updater: Arc<dyn Fn(Message) + Send + Sync + 'static>,
    ) -> Arc<dyn Fn(Message) + Send + Sync>;
}

pub struct VviewStore<'a, R: Reducer + 'static> {
    // actions: &'a mut R::Action,
    //updater: Box<dyn Fn()>,
    // sender: ActionSender<R::Action>,
    //state: Ref<'a, R::State>,
    state: &'a R::State,
    environment: &'a R::Environment,
    /// This state is kept separate so that it can be
    /// kept in a `UseRef` and is only created once
    runtime: &'a UseState<Rruntime<R>>,
}

impl<'a, R: Reducer> VviewStore<'a, R> {
    pub fn send(&self, action: R::Action) {
        //self.runtime.read().sender.send(action);
        self.runtime.get().sender.send(action);
    }

    pub fn sender(&self) -> ActionSender<R::Action> {
        //self.runtime.read().sender.clone()
        self.runtime.get().sender.clone()
    }
}

impl<'a, R: Reducer> std::ops::Deref for VviewStore<'a, R> {
    type Target = R::State;

    fn deref(&self) -> &Self::Target {
        self.state.deref()
    }
}

struct Rruntime<R: Reducer> {
    scope_id: usize,
    /// Send an action that will be processed afterwards
    sender: ActionSender<R::Action>,
    /// External events. Can be of a variety of notification mechamisms.
    /// They should all use the `updater` they were handed to notify
    /// that a new `Action` was generated
    // external_messages: Vec<Arc<dyn Fn(R::Message) + Send + Sync>>,
    // FIXME: Drop Action?
    child_senders: RwLock<fxhash::FxHashMap<usize, Rc<dyn Fn(R::Message)>>>,
    /// When a child drops, it uses this to notify the parent (with the scope id)
    // notify_drop: Option<Box<dyn Fn(usize)>>,
    /// Current subscriptions so they can be cleared on drop
    subscriptions: RwLock<Vec<AnySubscription>>,
}

impl<R: Reducer> Rruntime<R> {
    fn new(
        scope_id: usize,
        sender: ActionSender<R::Action>,
        // external_messages: Vec<Arc<dyn Fn(R::Message) + Send + Sync>>,
    ) -> Self {
        Self {
            scope_id,
            sender,
            // external_messages,
            child_senders: Default::default(),
            // notify_drop: None,
            subscriptions: Default::default(),
        }
    }
}

// impl<R: Reducer> Drop for Rruntime<R> {
//     fn drop(&mut self) {
//         println!("drop drop {}", self.scope_id);
//         if let Some(ref notifier) = self.notify_drop {
//             log::trace!("Dropping {self:?}");
//             notifier(self.scope_id)
//         }
//     }
// }

impl<R: Reducer> std::fmt::Debug for Rruntime<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rruntime")
            .field("scope_id", &self.scope_id)
            .field("subscriptions", &self.subscriptions.read().unwrap().len())
            .finish()
    }
}

#[derive(Clone)]
pub struct ActionSender<Action: Clone> {
    sender: flume::Sender<Action>,
    updater: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl<Action: Clone> ActionSender<Action> {
    pub fn send(&self, action: Action) {
        if let Err(e) = self.sender.send(action) {
            log::error!("Could not send action {e:?}");
        }
        (*self.updater)();
    }
}

impl<'a, ParentR: Reducer> VviewStore<'a, ParentR> {
    /// Host with a payload value. If `Value` changes, the state will be reset.
    /// The `Value` has to be `Clone` because we keep the last value around in order
    /// to compare it.
    pub fn host_with<
        ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>,
        T,
        Value: PartialEq + Clone + 'static,
    >(
        &'a self,
        cx: Scope<'a, T>,
        value: Value,
        state: impl Fn(Value) -> ChildR::State,
    ) -> VviewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        Drops::register(cx, "host_with");
        let (child_sender, child_receiver) = cx.use_hook(|| flume::unbounded());

        // Send the initial action once.
        cx.use_hook(|| {
            if let Some(initial_action) = ChildR::initial_action() {
                if let Err(e) = child_sender.send(initial_action) {
                    log::error!("Could not send initial action {e:?}");
                }
            }
        });

        let last = use_ref(cx, || value.clone());
        let reset_state = last.with(|last_rf| if last_rf.ne(&value) { true } else { false });

        let child_state = cx.use_hook(|| MaybeUninit::new(state(value.clone())));

        if reset_state {
            *last.write_silent() = value.clone();
            unsafe {
                *child_state.assume_init_mut() = state(value);
            }
            // Send the initial value again
            if let Some(initial) = ChildR::initial_action() {
                if let Err(e) = child_sender.send(initial) {
                    log::error!("Could not send reset initial action {e:?}");
                }
            }
        }

        self.host_internal(cx, child_state, child_sender, child_receiver, reset_state)
    }

    /// Host with a static state. This state will only change via `Action` messages
    pub fn host<ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>, T>(
        &'a self,
        cx: Scope<'a, T>,
        state: impl FnOnce() -> ChildR::State,
    ) -> VviewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        Drops::register(cx, "host");
        let child_state = cx.use_hook(|| MaybeUninit::new(state()));

        let (child_sender, child_receiver) = cx.use_hook(|| flume::unbounded());

        // Send the initial action once.
        cx.use_hook(|| {
            if let Some(initial_action) = ChildR::initial_action() {
                if let Err(e) = child_sender.send(initial_action) {
                    log::error!("Could not send initial action {e:?}");
                }
            }
        });

        self.host_internal(cx, child_state, child_sender, child_receiver, false)
    }

    // FIXME: Figure out the reset_state!
    fn host_internal<ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>, T>(
        &'a self,
        cx: Scope<'a, T>,
        child_state: &'a mut MaybeUninit<<ChildR as Reducer>::State>,
        child_sender: &'a mut flume::Sender<<ChildR as Reducer>::Action>,
        child_receiver: &'a mut flume::Receiver<<ChildR as Reducer>::Action>,
        reset: bool,
    ) -> VviewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        println!("host internal {:?}", cx.scope_id());
        Drops::register(cx, "host_internal");
        let environment = self.environment;

        // FIXME: reset_state
        // let reset_state = false;
        // if reset_state {
        //     *child_state.write_silent() = state();
        // }

        let scope_id = cx.scope_id().0;
        // let parent_runtime = self.runtime.read();
        let parent_sender = self.runtime.get().sender.clone();

        // Allow the child to send `DelegateMessage` messages
        // to the parent
        let delegate_sender = cx.use_hook(|| {
            move |action| {
                let Some(converted) = ChildR::from_child(action) else {
                return
            };
                parent_sender.send(converted);
            }
        });

        // And insert a way for the parent to communicate
        // with the child
        let updater = cx.schedule_update();
        let cloned_child_sender = child_sender.clone();
        cx.use_hook(|| {
            // self.runtime.with(|parent_runtime| {
            let sender = Rc::new(move |action| {
                let Some(child_message) = ChildR::to_child(action) else {
                    log::error!("failed to convert action for child");
                    return
                };
                if let Err(e) = cloned_child_sender.send(child_message) {
                    log::error!("Could not send action to parent {e:?}");
                }
                updater();
            });
            let mut s = self.runtime.child_senders.write().unwrap();
            if s.contains_key(&scope_id) {
                println!("ERROR: Hosted two child reducers in the same scope");
                println!("{}", include_str!("error_message.txt"));
            }
            s.insert(scope_id, sender);
            // });
        });

        let updater = cx.schedule_update();
        let cloned_child_sender = child_sender.clone();
        let updater = cx.use_hook(move || {
            Arc::new(move |action| {
                if let Err(e) = cloned_child_sender.send(action) {
                    log::error!("Could not send subscription {e:?}");
                }
                updater()
            })
        });

        // std::mem::forget(parent_runtime);

        let mut context: ReducerContext<
            'a,
            ChildR::Action,
            ChildR::Message,
            ChildR::DelegateMessage,
        > = ReducerContext {
            action_receiver: child_receiver,
            receivers: Default::default(),
            delegate_messages: &*delegate_sender,
            child_messages: Vec::new(),
            window: AppWindow::retrieve(&cx),
            timers: Default::default(),
            updater: updater.clone(),
        };

        // rrun(cx, context, child_state, external_events, reducer)

        // let child_view_store = run(context, reducer_state, environment_builder, initial_action, external_events, reducer)
        let view_store = rrun(cx, &mut context, child_state, environment, child_sender);

        // Register a drop handler to remove the child senders
        // and subscriptions
        Drops::register(cx, "host_internal pre-drop");
        let xparent_runtime = self.runtime.clone();
        let xchild_runtime = view_store.runtime.clone();
        Drops::action(cx, move || {
            log::info!("Inner Drop {scope_id}");

            let mut s = xparent_runtime.child_senders.write().unwrap();
            s.remove(&scope_id);

            // remove the child subscriptions
            let mut ct = xchild_runtime.subscriptions.write().unwrap();
            for sub in ct.iter() {
                log::info!("Inner Drop Subscription {scope_id}");
                sub.cancel();
            }
            ct.clear();
        });

        // let rrr = view_store.runtime.clone();

        // this is not optimal. if reset state is on, we also need to
        // get rid of the old subscriptions. this should kinda work via
        // drop, but dioxus doesn't drop if the `key: ...` changes.
        // so, at least get rid of the subscriptions here. other things
        // (like actions which are already queued) as well
        if reset {
            let mut s = self.runtime.child_senders.write().unwrap();
            s.remove(&scope_id);
            let mut ct = view_store.runtime.subscriptions.write().unwrap();
            for sub in ct.iter() {
                sub.cancel();
            }
            ct.clear();
            // FIXME: More?
        }

        // root(cx, view_store)
        view_store
    }
}

// pub trait HostChild

pub fn root<'a, R: Reducer, T>(
    cx: Scope<'a, T>,
    // reducer: R,
    environment: &'a R::Environment,
    state: impl FnOnce() -> R::State,
) -> VviewStore<'a, R>
where
    R: 'static,
{
    //let state = cx.use_hook(|| Cell::new(Some(R::initial_state(environment))));
    let state = cx.use_hook(|| MaybeUninit::new(state()));

    let (child_sender, action_receiver) = cx.use_hook(|| flume::unbounded());

    let delegate_sender = cx.use_hook(|| move |_| {});

    let updater = cx.schedule_update();
    let cloned_sender = child_sender.clone();

    let updater = cx.use_hook(move || {
        Arc::new(move |action| {
            if let Err(e) = cloned_sender.send(action) {
                log::error!("Could not send subscription {e:?}");
            }
            updater()
        })
    });

    let mut context: ReducerContext<'a, R::Action, R::Message, R::DelegateMessage> =
        ReducerContext {
            action_receiver,
            receivers: Default::default(),
            delegate_messages: &*delegate_sender,
            child_messages: Vec::new(),
            window: AppWindow::retrieve(&cx),
            timers: Default::default(),
            updater: updater.clone(),
        };

    let view_store = rrun(cx, &mut context, state, environment, child_sender);

    view_store
}

// FIXME: Reducers always send `Actions` which are then converted to `DelegateMessage` if they
// come from a Child or to `Message` if they come from a parent
pub struct ReducerContext<'a, Action, Message, DelegateMessage> {
    /// The queue of next actions to this reducer
    action_receiver: &'a flume::Receiver<Action>,
    /// Additional receivers from different sources
    receivers: FxHashMap<u64, flume::Receiver<Action>>,
    /// Delegate messages to the parent
    delegate_messages: &'a dyn Fn(DelegateMessage),
    /// Send messages to the child reducers
    child_messages: Vec<Rc<dyn Fn(Message)>>,
    /// Allow accessing the current window without `use_window`
    window: AppWindow<'a>,
    /// Currently running timers
    timers: FxHashMap<AnyHashable, tokio::task::JoinHandle<()>>,
    // Schedule an update
    updater: Arc<dyn Fn(Action) + Send + Sync>,
}

impl<'a, Action, Message: Clone, DelegateMessage> UpdaterContext<Action>
    for ReducerContext<'a, Action, Message, DelegateMessage>
{
    fn updater(&self) -> &Arc<dyn Fn(Action) + Send + Sync> {
        &self.updater
    }
}

impl<'a, Action, Message: Clone, DelegateMessage> MessageContext<Action, DelegateMessage, Message>
    for ReducerContext<'a, Action, Message, DelegateMessage>
{
    fn send_parent(&self, message: DelegateMessage) {
        (self.delegate_messages)(message);
    }

    fn send_children(&self, message: Message) {
        for child in self.child_messages.iter() {
            (*child)(message.clone());
        }
    }

    fn window(&self) -> &AppWindow {
        &self.window
    }
}

fn rrun<'a, T, R: Reducer + 'static>(
    cx: Scope<'a, T>,
    context: &mut ReducerContext<'a, R::Action, R::Message, R::DelegateMessage>,
    //state: &'a mut Cell<Option<R::State>>,
    state: &'a mut MaybeUninit<R::State>,
    environment: &'a R::Environment,
    action_sender: &'a mut flume::Sender<R::Action>,
) -> VviewStore<'a, R>
// where
//     R: 'static,
{
    let scope_id = cx.scope_id().0;
    let updater = cx.schedule_update();

    let sender = ActionSender {
        sender: action_sender.clone(),
        updater: updater.clone(),
    };

    // Sending new actions
    // let (action_receiver, action_sender) = cx.use_hook(|| {
    //     let (sender, receiver) = flume::unbounded::<R::Action>();
    //     let sender = ActionSender {
    //         sender,
    //         updater: updater.clone(),
    //     };
    //     (receiver, sender)
    // });

    let runtime: &UseState<Rruntime<R>> = use_state(cx, || {
        Rruntime::new(
            scope_id,
            ActionSender {
                sender: action_sender.clone(),
                updater: updater.clone(),
            },
        )
    });

    // the child senders are created later in the runtime (as the user interacts),
    // but once this code is called, they exist. so we can clone thme into the
    // parent so that they can be executed
    {
        let current_child_senders = runtime.child_senders.read().unwrap();
        if !current_child_senders.is_empty() {
            // Need to be wrapped so they convert from Message to Action
            context.child_messages = current_child_senders
                .values()
                .cloned()
                // .map(|e| Rc::new(move |message| e(message.into_action())))
                .collect();
        }
    }
    // let mut known_actions = cx.use_hook(|| {
    //     if let Some(initial_action) = R::initial_action() {
    //         vec![initial_action]
    //     } else {
    //         Vec::new()
    //     }
    // });
    let mut known_actions = Vec::new();

    // Get the initial action

    // Read all events that have been sent
    for action in context.action_receiver.try_iter() {
        known_actions.push(action);
    }
    // Read all external events
    // if let Some(ref receiver) = external_receiver {
    //     for action in receiver.try_iter() {
    //         known_actions.push(action);
    //     }
    // }

    // Read all other receiver events
    for receiver in context.receivers.values() {
        for action in receiver.try_iter() {
            known_actions.push(action);
        }
    }

    // set up the coroutine that handles async actions
    // FIXME: Only the root should create this coroutine?
    let cloned_sender = action_sender.clone();
    let coroutine = use_coroutine(
        cx,
        |mut rx: UnboundedReceiver<BoxFuture<'_, R::Action>>| async move {
            while let Some(task) = rx.next().await {
                let cloned_sender = cloned_sender.clone();

                tokio::task::spawn(async move {
                    let output = task.await;
                    if let Err(e) = cloned_sender.send(output) {
                        log::error!("Could not send coroutine result {e:?}");
                    }
                });
            }
        },
    );

    let eval = dioxus_desktop::use_eval(&cx);

    // Special handling for the delay action. This will return an Effect, not an Action, so we need
    // a way to handle running this code again once an effect has been created at a later point in time
    let later_effect: &UseRef<Option<InnerEffect<'_, R::Action>>> = use_ref(cx, || None);

    // Convert actions into effects and then handle in a loop
    let mut effects: Vec<_> = known_actions.drain(0..).map(InnerEffect::Action).collect();

    {
        let mut m = later_effect.write_silent();
        if let Some(e) = m.take() {
            effects.insert(0, e);
        }
    }

    if !effects.is_empty() {
        // let mut current_runtime = runtime.write_silent();

        let mut current_state = unsafe { state.assume_init_mut() };

        loop {
            let mut additions: Vec<InnerEffect<'_, R::Action>> = Vec::with_capacity(2);
            for effect in effects.drain(0..) {
                match effect {
                    InnerEffect::Future(fut) => {
                        coroutine.send(fut);
                    }
                    InnerEffect::Delay(d, e) => {
                        let cloned_later = later_effect.clone();
                        cx.push_future(async move {
                            tokio::time::sleep(d.into()).await;
                            let unwrapped = *e;
                            cloned_later.with_mut(|w| {
                                w.replace(unwrapped);
                            })
                        });
                    }
                    InnerEffect::Action(action) => {
                        let next =
                            R::reduce(&*context, action, &mut current_state, environment.deref());
                        additions.push(next.inner());
                        continue;
                    }
                    InnerEffect::Subscription(h) => {
                        let _ = runtime.get().subscriptions.write().map(|mut e| e.push(h));
                    }
                    InnerEffect::Multiple(mut v) => {
                        additions.append(&mut v);
                        continue;
                    }
                    InnerEffect::Nothing => (),
                    InnerEffect::Ui(s) => {
                        let eval = eval.clone();
                        cx.push_future(async move {
                            eval(s);
                        });
                    }
                    InnerEffect::UiFuture(fut) => {
                        let cloned_sender = sender.clone();
                        cx.push_future(async move {
                            if let Some(n) = fut.await {
                                cloned_sender.send(n);
                            }
                        });
                    }
                    InnerEffect::Timer(duration, action, id) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // let update_fn = context.cx().schedule_update();
                            let cloned_action = action;

                            {
                                // let mut runtime = runtime.write_silent();
                                let (sender, receiver) = flume::unbounded();
                                context.receivers.insert(id.id(), receiver);
                                // runtime
                                //     .receivers
                                //     .push(Some(Box::new(TimerEventReceiver::new(receiver))));

                                let cloned_updater = updater.clone();
                                context.timers.insert(
                                    id,
                                    tokio::spawn(async move {
                                        loop {
                                            tokio::time::sleep(
                                                tokio::time::Duration::from_secs_f64(
                                                    duration.as_secs_f64(),
                                                ),
                                            )
                                            .await;
                                            if let Err(e) = sender.send(cloned_action.clone()) {
                                                log::error!("Could not send timer {e:?}");
                                            }
                                            cloned_updater();
                                        }
                                    }),
                                )
                            };
                        }
                    }
                    InnerEffect::CancelTimer(id) => {
                        if let Some(n) = context.timers.remove(&id) {
                            n.abort();
                        }
                    }
                }
            }
            if !additions.is_empty() {
                effects.append(&mut additions);
                continue;
            }
            break;
        }

        // let is_equal = reducer_state.read().eq(&current_state);
        // if !is_equal {
        //     *reducer_state.write() = current_state;
        // }
    }

    // ?
    // let mut events = Vec::with_capacity(10);
    // for oreceiver in runtime.read().receivers.iter() {
    //     let Some(receiver) = oreceiver else { continue };
    //     if let Some(action) = receiver.receive() {
    //         events.push(action);
    //     }
    // }

    unsafe {
        VviewStore {
            state: state.assume_init_ref(),
            environment,
            runtime,
        }
    }
}

/// Simple Hashable
trait SimpleHashable {
    fn hashed(&self) -> u64;
}

impl<'a> SimpleHashable for &'a str {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl SimpleHashable for usize {
    fn hashed(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct AnyHashable(u64);

impl AnyHashable {
    pub fn id(&self) -> u64 {
        self.0
    }
}

impl<'a> From<&str> for AnyHashable {
    fn from(value: &str) -> Self {
        AnyHashable(value.hashed())
    }
}

impl<'a> From<usize> for AnyHashable {
    fn from(value: usize) -> Self {
        AnyHashable(value.hashed())
    }
}