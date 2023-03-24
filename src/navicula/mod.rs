#![allow(unused)]
pub mod event_receiver;
//pub mod menu;
//pub mod notify;
pub mod traits;
mod types;

use types::AppWindow;

use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    collections::{hash_map::DefaultHasher, HashMap},
    future::IntoFuture,
    hash::{Hash, Hasher},
    pin::Pin,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use dioxus::prelude::*;
use flume::Sender;
use futures_util::{future::BoxFuture, Future, FutureExt, StreamExt};

use itertools::Itertools;
// pub use notify::{Notifier, ScopeNotifyExt};

use self::{
    event_receiver::{
        ActionEventReceiver, AppEvent, DefaultEventReceiver, EventReceiver, TimerEventReceiver,
    },
    traits::AnyHashable,
};

pub enum Effect<'a, A> {
    Future(BoxFuture<'a, A>),
    Action(A),
    // Subscription(Box<dyn EventReceiver<A>>),
    Subscription(flume::Receiver<A>, AnyHashable),
    Multiple(Vec<Effect<'a, A>>),
    Nothing,
    /// Execute the following javascript, ignore the result
    Ui(String),
    /// Execute the following javascript, but also get the result back and convert into an action
    UiFuture(Pin<Box<dyn Future<Output = Option<A>> + 'static>>),
    /// Maybe a better solution? Timer. the u64 parameter can be used to cancel it again
    Timer(Duration, A, AnyHashable),
    CancelTimer(AnyHashable),
}

impl<'a, A> Effect<'a, A> {
    pub fn merge2(a: Effect<'a, A>, b: Effect<'a, A>) -> Self {
        Self::Multiple(vec![a, b])
    }

    pub fn merge3(a: Effect<'a, A>, b: Effect<'a, A>, c: Effect<'a, A>) -> Self {
        Self::Multiple(vec![a, b, c])
    }

    pub fn merge4(a: Effect<'a, A>, b: Effect<'a, A>, c: Effect<'a, A>, d: Effect<'a, A>) -> Self {
        Self::Multiple(vec![a, b, c, d])
    }

    pub fn merge5(
        a: Effect<'a, A>,
        b: Effect<'a, A>,
        c: Effect<'a, A>,
        d: Effect<'a, A>,
        e: Effect<'a, A>,
    ) -> Self {
        Self::Multiple(vec![a, b, c, d, e])
    }

    pub fn merge6(
        a: Effect<'a, A>,
        b: Effect<'a, A>,
        c: Effect<'a, A>,
        d: Effect<'a, A>,
        e: Effect<'a, A>,
        f: Effect<'a, A>,
    ) -> Self {
        Self::Multiple(vec![a, b, c, d, e, f])
    }

    // pub fn timer(duration: Duration, action: A, identifier: impl Hash) -> Effect<'a, A> {
    //     let mut hasher = DefaultHasher::default();
    //     identifier.hash(&mut hasher);
    //     Effect::Timer(duration, action, hasher.finish())
    // }
}

#[derive(Clone)]
pub struct ViewStore<'a, Action: Clone + 'static, State: Clone, Environment: Clone> {
    collection: &'a UseRef<Vec<Action>>,
    state: State,
    environment: &'a Environment,
    runtime: &'a UseRef<Runtime<Action>>,
    updater: Arc<dyn Fn() + Send + Sync>,
}

impl<'a, Action: Clone, State: Clone, Environment: Clone>
    ViewStore<'a, Action, State, Environment>
{
    pub fn send(&self, action: Action) {
        self.collection.write().push(action);
    }
}

impl<'a, Action: Clone, State: Clone, Environment: Clone> std::ops::Deref
    for ViewStore<'a, Action, State, Environment>
{
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<'a, HostAction: Clone, HostState: Clone, Environment: Clone + 'static>
    ViewStore<'a, HostAction, HostState, Environment>
{
    pub fn hoist<
        'b,
        T,
        PublicChildAction: Send + Clone + std::fmt::Debug + 'static,
        ChildAction: Send + Clone + std::fmt::Debug + 'static,
        ChildState: Clone + PartialEq + std::fmt::Debug + 'static,
        RF,
    >(
        &self,
        cx: Scope<'b, T>,
        state_builder: impl Fn() -> ChildState,
        initial_action: Option<ChildAction>,
        drop_action: Option<PublicChildAction>,
        mapper: impl Fn(PublicChildAction) -> HostAction + 'static,
        reverse_mapper: impl Fn(HostAction) -> Option<ChildAction> + 'static,
        reducer: RF,
        reset_state: bool,
    ) -> ViewStore<'a, ChildAction, ChildState, Environment>
    where
        RF: Fn(
            &ChildContext<'b, T, PublicChildAction, ChildAction>,
            ChildAction,
            &mut ChildState,
            Environment,
        ) -> Effect<'static, ChildAction>,
        'b: 'a,
    {
        // if the input to our state changed, we redo it
        let use_child_state = use_ref(cx, || state_builder());

        // Reset the state if needed
        if reset_state {
            let o = state_builder();
            *use_child_state.write_silent() = o;
        }

        // Insert a way to recieve communication from the child
        let (sender, update_fn, waker) = cx.use_hook(|| {
            let (sender, receiver) = flume::unbounded();

            let update_fn = self.updater.clone();

            let runtime_receiver = Box::new(ActionEventReceiver::new(receiver));
            let mut runtime_mut = self.runtime.write_silent();
            runtime_mut
                .child_recievers
                .insert(cx.scope_id().0, runtime_receiver.clone());

            let cloned_sender = sender.clone();
            let cloned_update = update_fn.clone();
            let waker = Rc::new(move |action: PublicChildAction| {
                if let Err(e) = cloned_sender.send(mapper(action)) {
                    log::error!("{e}");
                }
                cloned_update();
            });

            (sender, update_fn, waker)
        });

        // And a way to send communication up to the child
        let cloned_update = update_fn.clone();
        let parent_receiver = cx.use_hook(|| {
            let (sender, receiver) = flume::unbounded();
            let mut runtime_mut = self.runtime.write_silent();
            runtime_mut.child_senders.insert(
                cx.scope_id().0,
                Rc::new(move |action| {
                    if let Some(n) = reverse_mapper(action) {
                        sender.send(n);
                        cloned_update();
                    }
                }),
            );
            Box::new(ActionEventReceiver::new(receiver))
        });

        // And a way to communicate from a callback
        let (sender, reciever) = cx.use_hook(|| flume::unbounded());
        let movable_sender = sender.clone();
        let updater = cx.schedule_update();

        let window = AppWindow::retrieve(cx);

        let child_context = ChildContext {
            cx,
            parent_handler: waker.clone(),
            child_senders: vec![],
            waker: Rc::new(move |action| {
                movable_sender.send(action);
                updater();
            }),
            window,
        };

        let drop_state: &UseRef<Option<DropState>> = use_ref(cx, || None);

        // send an action on drop
        if let Some(drop) = drop_action {
            let cloned_waker = waker.clone();
            let mut s = drop_state.write_silent();
            s.replace(DropState(Box::new(move || {
                log::info!("dropping {drop:?}");
                (*cloned_waker)(drop.clone());
            })));
        }

        let view_store = run(
            child_context,
            use_child_state,
            || self.environment.clone(),
            initial_action.clone(),
            || vec![parent_receiver.clone()],
            // || vec![],
            reducer,
        );

        cx.use_hook(|| {
            // we should only need to do this once
            let parent_runtime = self.runtime.clone();
            let notify_drop = Rc::new(move |id: usize| {
                let mut rt = parent_runtime.write_silent();
                rt.child_recievers.remove(&id);
                rt.child_senders.remove(&id);
            });
            view_store.runtime.write_silent().notify_drop = Some(notify_drop);
        });

        if reset_state {
            if let Some(ref i) = initial_action {
                view_store.send(i.clone());
            }
        }

        view_store
    }
}

pub struct ChildContext<'a, T, ChildAction: Clone + 'static, HostAction> {
    cx: dioxus::core::Scope<'a, T>,
    child_senders: Vec<Rc<dyn Fn(HostAction)>>,
    parent_handler: Rc<dyn Fn(ChildAction)>,
    waker: Rc<dyn Fn(ChildAction)>,
    window: AppWindow<'a>,
}

impl<'a, T, ChildAction: Clone, HostAction> ChildContext<'a, T, ChildAction, HostAction> {
    pub fn cx(&self) -> dioxus::core::Scope<'a, T> {
        self.cx
    }

    fn child_senders(&self) -> &[Rc<dyn Fn(HostAction)>] {
        &self.child_senders
    }

    fn set_senders(&mut self, senders: Vec<Rc<dyn Fn(HostAction)>>) {
        self.child_senders = senders;
    }

    /// Returns a waker for the current Scope. Calling it will trigger
    /// the reducer with the given action. Can be used for callbacks
    pub fn waker(&self) -> Rc<dyn Fn(ChildAction)> {
        self.waker.clone()
    }

    pub fn window(&self) -> &AppWindow {
        &self.window
    }

    pub fn eval_ui(
        &self,
        js: impl AsRef<str>,
    ) -> Pin<Box<dyn Future<Output = Option<HostAction>> + 'static>> {
        let fut = self.window.eval(js.as_ref()).into_future();
        Box::pin(async move {
            fut.await;
            None
        })
    }

    pub fn eval_future(
        &self,
        js: impl AsRef<str>,
        action: impl Fn(serde_json::Value) -> Result<HostAction, String> + 'static,
    ) -> Pin<Box<dyn Future<Output = Option<HostAction>> + 'static>> {
        let fut = self.window.eval(js.as_ref()).into_future();
        Box::pin(async move {
            let v = fut.await;
            match v {
                Ok(v) => match action(v) {
                    Ok(n) => Some(n),
                    Err(e) => {
                        log::error!("UiFuture Error: {e:?}");
                        None
                    }
                },
                Err(e) => {
                    log::error!("UiFuture Error: {e:?}");
                    None
                }
            }
        })
    }
}

impl<'a, T, ChildAction: Clone, HostAction: Clone> ChildContext<'a, T, ChildAction, HostAction> {
    /// Send a message to all current child reducers
    pub fn send_children(&self, action: HostAction) {
        self.child_senders().iter().for_each(|sender| {
            sender(action.clone());
        });
    }
}

impl<'a, T, ChildAction: Clone, HostAction> ChildContext<'a, T, ChildAction, HostAction> {
    /// Send a message to the parent reducer
    pub fn send(&self, action: ChildAction) {
        (*self.parent_handler)(action)
    }
}

// For the root reducer
pub fn root<
    'a,
    T,
    Action: Send + Clone + std::fmt::Debug + 'static,
    State: Clone + PartialEq + std::fmt::Debug + 'static,
    Environment: Clone + 'static,
    RF,
>(
    cx: Scope<'a, T>,
    state_builder: impl Fn() -> State,
    environment_builder: impl Fn() -> Environment,
    initial_action: Option<Action>,
    drop_action: Option<Action>,
    external_events: impl Fn() -> Vec<Box<dyn EventReceiver<Action>>>,
    parent_handler: Rc<dyn Fn(Action)>,
    reducer: RF,
) -> ViewStore<Action, State, Environment>
where
    RF: Fn(
        &ChildContext<'a, T, Action, Action>,
        Action,
        &mut State,
        Environment,
    ) -> Effect<'static, Action>,
{
    let window = AppWindow::retrieve(cx);
    let (sender, reciever) = cx.use_hook(|| flume::unbounded());

    // send an action on drop
    let drop_state: &UseRef<Option<DropState>> = use_ref(cx, || None);
    if let Some(drop) = drop_action {
        let handler = parent_handler.clone();
        let mut s = drop_state.write_silent();
        s.replace(DropState(Box::new(move || {
            (*handler)(drop.clone());
        })));
    }

    let movable_sender = sender.clone();
    let updater = cx.schedule_update();
    let root_context = ChildContext {
        cx,
        child_senders: vec![],
        parent_handler,
        waker: Rc::new(move |action| {
            movable_sender.send(action);
            updater();
        }),
        window,
    };
    let use_root_state = use_ref(cx, state_builder);

    crate::navicula::run(
        root_context,
        use_root_state,
        environment_builder,
        initial_action,
        move || {
            let mut evts = external_events();
            evts.push(Box::new(ActionEventReceiver::new(reciever.clone())));
            evts
        },
        reducer,
    )
}

struct Runtime<Action> {
    scope_id: usize,
    receivers: Vec<Option<Box<dyn EventReceiver<Action>>>>,
    child_recievers: HashMap<usize, Box<dyn EventReceiver<Action>>>,
    child_senders: HashMap<usize, Rc<dyn Fn(Action)>>,
    #[cfg(target_arch = "wasm32")]
    timers: HashMap<u64, usize>,
    #[cfg(not(target_arch = "wasm32"))]
    timers: HashMap<u64, tokio::task::JoinHandle<()>>,
    /// When a child drops, it uses this to notify the parent (with the scope id)
    notify_drop: Option<Rc<dyn Fn(usize)>>,
}

impl<Action> Runtime<Action> {
    fn new(scope_id: usize, receivers: Vec<Box<dyn EventReceiver<Action>>>) -> Self {
        Self {
            scope_id,
            receivers: receivers.into_iter().map(|e| Some(e)).collect(),
            child_recievers: Default::default(),
            child_senders: Default::default(),
            timers: HashMap::new(),
            notify_drop: None,
        }
    }
}

impl<Action> Drop for Runtime<Action> {
    fn drop(&mut self) {
        if let Some(ref notifier) = self.notify_drop {
            notifier(self.scope_id)
        }
    }
}

fn run<
    'a,
    T: 'a,
    Action: Send + Clone + std::fmt::Debug,
    ParentAction: Clone,
    State: Clone + PartialEq + std::fmt::Debug + 'static,
    Environment: Clone + 'static,
    RF,
>(
    mut context: ChildContext<'a, T, ParentAction, Action>,
    reducer_state: &'a UseRef<State>,
    environment_builder: impl Fn() -> Environment,
    initial_action: Option<Action>,
    external_events: impl Fn() -> Vec<Box<dyn EventReceiver<Action>>>,
    reducer: RF,
) -> ViewStore<'a, Action, State, Environment>
where
    RF: Fn(
        &ChildContext<'a, T, ParentAction, Action>,
        Action,
        &mut State,
        Environment,
    ) -> Effect<'static, Action>,
{
    let (future_receiver, future_sender) = context.cx().use_hook(|| {
        let (future_sender, future_receiver) = flume::unbounded();
        let future_updater = context.cx().schedule_update();
        let future_sender = Arc::new(move |action: Action| {
            future_sender.send(action);
            future_updater();
        });
        (future_receiver, future_sender)
    });

    let mut recievers = external_events();
    recievers.push(Box::new(ActionEventReceiver::new(future_receiver.clone())));

    // This is our internal private state, the navicula runtime
    let scope_id = context.cx().scope_id().0;
    let runtime: &UseRef<Runtime<Action>> =
        use_ref(context.cx(), || Runtime::new(scope_id, recievers));

    // the child senders are created later in the runtime (as the user interacts),
    // but once this code is called, they exist. so we can clone thme into the
    // parent so that they can be executed
    context.set_senders(runtime.read().child_senders.values().cloned().collect());

    // This is the state that is being processed by the reducers
    // let reducer_state = use_state(context.cx(), || state);
    let environment = use_state(context.cx(), || environment_builder());

    // In order to use `initial_actions` only once, `known_actions` is a
    // `use_ref`
    let known_actions = use_ref(context.cx(), || {
        initial_action.map(|a| vec![a]).unwrap_or_default()
    });

    #[cfg(target_arch = "wasm32")]
    let eval = dioxus_web::use_eval(context.cx());

    #[cfg(not(target_arch = "wasm32"))]
    let eval = dioxus_desktop::use_eval(context.cx());

    let cloned = known_actions.clone();
    let cloned_sender = future_sender.clone();
    let coroutine = use_coroutine(
        context.cx(),
        |mut rx: UnboundedReceiver<BoxFuture<'_, Action>>| async move {
            while let Some(task) = rx.next().await {
                let cloned_sender = cloned_sender.clone();

                tokio::task::spawn(async move {
                    let output = task.await;
                    (cloned_sender.clone())(output);
                });
            }
        },
    );

    let mut events = Vec::with_capacity(10);
    for oreceiver in runtime.read().receivers.iter() {
        let Some(receiver) = oreceiver else { continue };
        if let Some(action) = receiver.receive() {
            events.push(action);
        }
    }

    for receiver in runtime.read().child_recievers.values() {
        if let Some(action) = receiver.receive() {
            events.push(action);
        }
    }
    if !events.is_empty() {
        known_actions.write_silent().append(&mut events);
    }

    let update_fn = context.cx().schedule_update();
    /*
    FIXME: menu handlers as an outside effect
    use_future(context.cx(), (), |_| async {
        menu::setup_menu_handler(update_fn);
    });

    if let Some(context_action) = menu::resolve_current_action() {
        known_actions.write_silent().push(context_action);
    }
     */

    // take out the actions
    let mut effects: Vec<_> = known_actions
        .write_silent()
        .drain(0..)
        .map(Effect::Action)
        .collect();
    if !effects.is_empty() {
        let mut current_state = reducer_state.read().clone();

        let env_clone = environment.get().clone();

        loop {
            let mut additions: Vec<Effect<'_, Action>> = Vec::with_capacity(2);
            for effect in effects.drain(0..) {
                match effect {
                    Effect::Future(fut) => coroutine.send(fut),
                    Effect::Action(action) => {
                        additions.push(reducer(
                            &context,
                            action,
                            &mut current_state,
                            env_clone.clone(),
                        ));
                        continue;
                    }
                    // Effect::Subscription(receiver) => {
                    //     runtime.write_silent().receivers.push(Some(receiver));
                    //     // runtime.with_mut(|r| r.receivers.push(Some(receiver)))
                    // }
                    Effect::Subscription(_, _) => {}
                    Effect::Multiple(mut v) => {
                        additions.append(&mut v);
                        continue;
                    }
                    Effect::Nothing => (),
                    Effect::Ui(s) => {
                        let eval = eval.clone();
                        context.cx().push_future(async move {
                            eval(s);
                        });
                    }
                    Effect::UiFuture(fut) => {
                        // FIXME: Always use push_future for this?
                        // or have a `ui_after` action?
                        // let eval = eval.clone();
                        // let update_fn = context.cx().schedule_update();
                        let clo = known_actions.clone();
                        //println!("add task for {}", context.cx.scope_id().0);
                        context.cx().push_future(async move {
                            //let id = "status_timeline_store_data_ebou::components::status_timeline::providers::public::PublicTimelineProvider";
                            //let px = eval(format!("document.getElementById('{id}').scollTop")).into_future().await;
                            // match px {
                            //     Ok(o) => {
                            //         println!("{:?}", o.as_f64());
                            //     }
                            //     Err(e) => {
                            //         println!("EEE {e:?}")
                            //     }
                            // }
                            if let Some(n) = fut.await {
                                clo.write().push(n);
                            }
                        });
                    }
                    Effect::Timer(duration, action, id) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let update_fn = context.cx().schedule_update();
                            let cloned_action = action;

                            {
                                let mut runtime = runtime.write_silent();
                                let (sender, receiver) = flume::unbounded();
                                runtime
                                    .receivers
                                    .push(Some(Box::new(TimerEventReceiver::new(receiver))));

                                runtime.timers.insert(
                                    id.id(),
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
                                            update_fn();
                                        }
                                    }),
                                )
                            };
                        }
                    }
                    Effect::CancelTimer(id) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let mut runtime = runtime.write_silent();
                            if let Some(n) = runtime.timers.remove(&id.id()) {
                                n.abort();
                            }
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

        let is_equal = reducer_state.read().eq(&current_state);
        if !is_equal {
            *reducer_state.write() = current_state;
        }
    }

    let current = reducer_state.read().clone();
    let updater = context.cx().schedule_update();
    ViewStore {
        collection: &known_actions,
        state: current,
        environment: &environment,
        runtime,
        updater,
    }
}

pub fn make_future<'b, T, FU, M, Action: Clone>(fu: FU, mapper: M) -> Effect<'b, Action>
where
    FU: Future<Output = T> + Send + 'b,
    M: FnOnce(T) -> Action + Send + Sync + 'b,
{
    Effect::Future(Box::pin(async move { fu.map(move |o| mapper(o)).await }))
}

pub fn make_debounce_future<'b, T, FU, M, Action: Clone>(
    duration: Duration,
    bouncer: SearchDebounce,
    fu: FU,
    mapper: M,
) -> Effect<'b, Action>
where
    FU: Future<Output = T> + Send + 'b,
    M: Fn(Option<T>) -> Action + Send + Sync + 'b,
{
    Effect::Future(Box::pin(async move {
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(duration.as_secs_f64())).await;

        // if we were debounced, return
        let val = bouncer.0.load(std::sync::atomic::Ordering::Relaxed);
        if val {
            return mapper(None);
        }

        let o = fu.map(move |o| mapper(Some(o))).await;
        o
    }))
}

pub fn make_delayed_future<'b, T, FU, M, Action: Clone>(
    delay: Duration,
    fu: FU,
    mapper: M,
) -> Effect<'b, Action>
where
    FU: Future<Output = T> + Send + 'b,
    M: FnOnce(Option<T>) -> Action + Send + Sync + 'b,
{
    Effect::Future(Box::pin(async move {
        #[cfg(not(target_arch = "wasm32"))]
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(delay.as_secs_f64())).await;
        fu.map(move |o| mapper(Some(o))).await
    }))
}

#[derive(Clone, Default, Debug)]
pub struct SearchDebounce(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl PartialEq for SearchDebounce {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}

impl Eq for SearchDebounce {}

impl SearchDebounce {
    pub fn cancel(&self) {
        self.0.swap(true, std::sync::atomic::Ordering::SeqCst);
    }
}

// Thi sis kind of screaming for a trait
pub fn mount_childstore<
    'a,
    T: 'a,
    ChildAction: Clone + Send + std::fmt::Debug,
    ParentAction: Clone,
    ParentState: Clone + Eq + PartialEq,
    InputState: Eq + PartialEq + Clone + 'static,
    Environment: Clone + 'static,
    State: Clone + Eq + PartialEq + std::fmt::Debug + 'static,
    RF,
>(
    cx: Scope<'a, T>,
    parent_store: &crate::navicula::ViewStore<'a, ParentAction, ParentState, Environment>,
    input_state: InputState,
    state_mapper: impl Fn(&InputState) -> State,
    action_mapper: impl Fn(ChildAction) -> ParentAction + 'static,
    reverse_action_mapper: impl Fn(ParentAction) -> Option<ChildAction> + 'static,
    initial_action: Option<ChildAction>,
    drop_action: Option<ChildAction>,
    reducer: RF,
) -> ViewStore<'a, ChildAction, State, Environment>
where
    RF: Fn(
        &ChildContext<'a, T, ChildAction, ChildAction>,
        ChildAction,
        &mut State,
        Environment,
    ) -> Effect<'static, ChildAction>,
{
    let last = use_ref(cx, || input_state.clone());
    let reset_state = {
        let last_rf = last.read();
        if last_rf.ne(&input_state) {
            true
        } else {
            false
        }
    };
    if reset_state {
        *last.write_silent() = input_state.clone();
    }
    let store = parent_store.hoist(
        cx,
        || state_mapper(&input_state),
        initial_action,
        drop_action,
        action_mapper,
        reverse_action_mapper,
        reducer,
        reset_state,
    );

    store
}

struct DropState(Box<dyn Fn()>);

impl Drop for DropState {
    fn drop(&mut self) {
        (*self.0)()
    }
}
