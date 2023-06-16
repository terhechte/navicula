use super::anyhashable::AnyHashable;
use dioxus::prelude::*;
use futures_util::{future::BoxFuture, StreamExt};
use fxhash::FxHashMap;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::{rc::Rc, sync::Arc};

use super::publisher::Subscription;
use super::types::{MessageContext, UpdaterContext};
use super::{
    effect::InnerEffect,
    reducer::{ChildReducer, Reducer},
    runtime::Runtime,
    types::{ActionSender, AppWindow},
    viewstore::ViewStore,
};

impl<'a, ParentR: Reducer> ViewStore<'a, ParentR> {
    /// Host with a payload value. If `Value` changes, the state will be reset.
    /// The `Value` has to be `Clone` because we keep the last value around in order
    /// to compare it.
    pub fn host_with<
        'b,
        ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>,
        T,
        Value: PartialEq + Clone + 'static,
    >(
        &'a self,
        cx: Scope<'a, T>,
        value: &'b Value,
        state: impl Fn(Value) -> ChildR::State,
    ) -> ViewStore<'a, ChildR>
    where
        ChildR: 'static,
        ParentR: 'static,
    {
        let (child_sender, child_receiver) = cx.use_hook(flume::unbounded);

        // Send the initial action once.
        cx.use_hook(|| {
            if let Some(initial_action) = ChildR::initial_action() {
                if let Err(e) = child_sender.send(initial_action) {
                    log::error!("Could not send initial action {e:?}");
                }
            }
        });

        let last = use_ref(cx, move || value.clone());
        let reset_state = last.with(|last_rf| last_rf.ne(value));

        let child_state = cx.use_hook(|| MaybeUninit::new(state(value.clone())));

        if reset_state {
            *last.write_silent() = value.clone();
            unsafe {
                *child_state.assume_init_mut() = state(value.clone());
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
    ) -> ViewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        let child_state = cx.use_hook(|| MaybeUninit::new(state()));

        let (child_sender, child_receiver) = cx.use_hook(flume::unbounded);

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

    fn host_internal<ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>, T>(
        &'a self,
        cx: Scope<'a, T>,
        child_state: &'a mut MaybeUninit<<ChildR as Reducer>::State>,
        child_sender: &'a mut flume::Sender<<ChildR as Reducer>::Action>,
        child_receiver: &'a mut flume::Receiver<<ChildR as Reducer>::Action>,
        reset: bool,
    ) -> ViewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        let environment = self.environment;

        let scope_id = cx.scope_id().0;
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
            let sender = Rc::new(move |action| {
                let Some(child_message) = ChildR::to_child(action) else {
                    return
                };
                if let Err(e) = cloned_child_sender.send(child_message) {
                    log::error!("Could not send action to child {e:?}");
                }
                updater();
            });
            let mut s = self.runtime.child_senders.write().unwrap();
            if s.contains_key(&scope_id) {
                println!("ERROR: Hosted two child reducers in the same scope");
                println!("{}", include_str!("error_message.txt"));
            }
            s.insert(scope_id, sender);
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
            window: AppWindow::retrieve(cx),
            timers: Default::default(),
            updater: updater.clone(),
        };

        let view_store = run_reducer(cx, &mut context, child_state, environment, child_sender);

        // Register a drop handler to remove the child senders
        // and subscriptions
        let xparent_runtime = self.runtime.clone();
        let xchild_runtime = view_store.runtime.clone();
        Drops::action(cx, move || {
            let mut s = xparent_runtime.child_senders.write().unwrap();
            s.remove(&scope_id);

            // remove the child subscriptions
            let mut ct = xchild_runtime.subscriptions.write().unwrap();
            for sub in ct.iter() {
                sub.cancel();
            }
            ct.clear();
        });

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
        }

        view_store
    }
}

pub fn root<'a, R: Reducer, T>(
    cx: Scope<'a, T>,
    // reducer: R,
    receivers: &[flume::Receiver<R::Action>],
    environment: &'a R::Environment,
    state: impl FnOnce() -> R::State,
) -> ViewStore<'a, R>
where
    R: 'static,
{
    let state = cx.use_hook(|| MaybeUninit::new(state()));

    let (child_sender, action_receiver) = cx.use_hook(flume::unbounded);

    cx.use_hook(|| {
        if let Some(initial_action) = R::initial_action() {
            if let Err(e) = child_sender.send(initial_action) {
                log::error!("Could not send initial action {e:?}");
            }
        }
    });

    // go through the recievers and pull all messages in
    for r in receivers {
        for entry in r.try_iter() {
            if let Err(n) = child_sender.send(entry) {
                log::error!("Could not send to parent: {n:?}");
            };
        }
    }

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
            window: AppWindow::retrieve(cx),
            timers: Default::default(),
            updater: updater.clone(),
        };

    let view_store = run_reducer(cx, &mut context, state, environment, child_sender);

    view_store
}

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

    fn window(&self) -> &AppWindow {
        &self.window
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

    fn children(&self) -> Vec<std::rc::Rc<dyn Fn(Message)>> {
        self.child_messages.clone()
    }
}

fn run_reducer<'a, T, R: Reducer + 'static>(
    cx: Scope<'a, T>,
    context: &mut ReducerContext<'a, R::Action, R::Message, R::DelegateMessage>,
    state: &'a mut MaybeUninit<R::State>,
    environment: &'a R::Environment,
    action_sender: &'a mut flume::Sender<R::Action>,
) -> ViewStore<'a, R> {
    let scope_id = cx.scope_id().0;
    let updater = cx.schedule_update();

    let sender = ActionSender {
        sender: action_sender.clone(),
        updater: updater.clone(),
    };

    let runtime: &UseState<Runtime<R>> = use_state(cx, || {
        Runtime::new(
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
            context.child_messages = current_child_senders.values().cloned().collect();
        }
    }

    let mut known_actions = Vec::new();

    // Read all events that have been sent
    for action in context.action_receiver.try_iter() {
        known_actions.push(action);
    }

    // Read all other receiver events
    for receiver in context.receivers.values() {
        for action in receiver.try_iter() {
            known_actions.push(action);
        }
    }

    // set up the coroutine that handles async actions
    let cloned_sender = sender.clone();
    let coroutine = use_coroutine(
        cx,
        |mut rx: UnboundedReceiver<BoxFuture<'_, R::Action>>| async move {
            while let Some(task) = rx.next().await {
                let cloned_sender = cloned_sender.clone();

                tokio::task::spawn(async move {
                    let output = task.await;
                    cloned_sender.send(output);
                });
            }
        },
    );

    let eval = dioxus_desktop::use_eval(cx);

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
        let current_state = unsafe { state.assume_init_mut() };

        loop {
            let mut additions: Vec<InnerEffect<'_, R::Action>> = Vec::with_capacity(2);
            for effect in effects.drain(0..) {
                match effect {
                    InnerEffect::Future(fut) => {
                        coroutine.send(fut);
                    }
                    InnerEffect::FireForget(fut) => {
                        tokio::spawn(async move { fut.await });
                    }
                    InnerEffect::Delay(d, e) => {
                        let cloned_later = later_effect.clone();
                        cx.push_future(async move {
                            tokio::time::sleep(d).await;
                            let unwrapped = *e;
                            cloned_later.with_mut(|w| {
                                w.replace(unwrapped);
                            })
                        });
                    }
                    InnerEffect::Action(action) => {
                        let next = R::reduce(&*context, action, current_state, environment.deref());
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
                    InnerEffect::UiFuture(js, action) => {
                        let cloned = sender.clone();
                        let eval = eval.clone();
                        cx.push_future(async move {
                            let result = eval(js).await;
                            match result {
                                Ok(r) => {
                                    let result = action(r);
                                    if let Some(r) = result {
                                        cloned.send(r);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Could not send future {e:?}");
                                }
                            }
                        });
                    }
                    InnerEffect::Timer(duration, action, id) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let cloned_action = action;

                            {
                                // let (sender, receiver) = cx.use_hook(|| flume::unbounded());
                                // context.receivers.insert(id.id(), receiver.clone());

                                // let cloned_updater = updater.clone();
                                let cloned_sender = sender.clone();
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
                                            cloned_sender.send(cloned_action.clone());
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
    }

    unsafe {
        ViewStore {
            state: state.assume_init_ref(),
            environment,
            runtime,
        }
    }
}

pub struct Drops(Box<dyn Fn()>);

impl Drops {
    pub fn action<T>(cx: Scope<'_, T>, a: impl Fn() + 'static) {
        let boxed = Box::new(a);
        cx.use_hook(move || Drops(boxed));
    }
}

impl Drop for Drops {
    fn drop(&mut self) {
        (self.0)()
    }
}
