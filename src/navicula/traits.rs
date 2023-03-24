use dioxus::prelude::*;
use futures_util::Future;
use futures_util::{future::BoxFuture, StreamExt};
use fxhash::FxHashMap;
use std::any::Any;
use std::cell::{Cell, Ref, RefMut};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::{rc::Rc, sync::Arc};

use async_trait::async_trait;

// view store generic over a reducer?
use super::{run, types::AppWindow, Effect, ViewStore};

pub trait Reducer {
    // type Context = ReducerContext<Self::Action, Self::Message, Self::DelegateMessage>;
    /// A reducer can be messaged from a parent.
    /// `Messages` are send parent to child
    type Message: Clone + IntoAction<Self::Action>;

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
        context: &'a ReducerContext<'a, Self::Action, Self::Message, Self::DelegateMessage>,
        action: Self::Action,
        state: &'a mut Self::State,
        environment: &'a Self::Environment,
    ) -> Effect<'b, Self::Action>;

    /// Define the initial action when the reducer starts up
    fn initial_action() -> Option<Self::Action>;

    /// Provide the initial state
    fn initial_state(environment: &Self::Environment) -> Self::State;

    // Provide the environment
    // fn environment(&self) -> &Self::Environment;
}

pub trait ChildReducer<Parent: Reducer>: Reducer {
    // type Parent: Reducer;

    fn to_child(message: <Parent as Reducer>::Message) -> Option<<Self as Reducer>::Action>;

    fn from_child(message: <Self as Reducer>::DelegateMessage) -> Option<Parent::Action>;
}

pub trait IntoAction<Action> {
    fn into_action(self) -> Action;
}

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

pub struct VviewStore<'a, R: Reducer> {
    // actions: &'a mut R::Action,
    //updater: Box<dyn Fn()>,
    // sender: ActionSender<R::Action>,
    //state: Ref<'a, R::State>,
    state: &'a R::State,
    environment: &'a R::Environment,
    // FIXME: Myabe mutable, for child-senders
    runtime: &'a UseRef<Rruntime<R>>,
}

impl<'a, R: Reducer> VviewStore<'a, R> {
    pub fn send(&self, action: R::Action) {
        self.runtime.read().sender.send(action);
    }
}

impl<'a, R: Reducer> std::ops::Deref for VviewStore<'a, R> {
    type Target = R::State;

    fn deref(&self) -> &Self::Target {
        self.state.deref()
    }
}

struct Rruntime<R: Reducer> {
    /// Send an action that will be processed afterwards
    sender: ActionSender<R::Action>,
    /// External events. Can be of a variety of notification mechamisms.
    /// They should all use the `updater` they were handed to notify
    /// that a new `Action` was generated
    // external_messages: Vec<Arc<dyn Fn(R::Message) + Send + Sync>>,
    // FIXME: Drop Action?
    child_senders: fxhash::FxHashMap<usize, Rc<dyn Fn(R::Message)>>,
    /// When a child drops, it uses this to notify the parent (with the scope id)
    notify_drop: Option<Box<dyn Fn(usize)>>,
}

impl<R: Reducer> Rruntime<R> {
    fn new(
        sender: ActionSender<R::Action>,
        // external_messages: Vec<Arc<dyn Fn(R::Message) + Send + Sync>>,
    ) -> Self {
        Self {
            sender,
            // external_messages,
            child_senders: Default::default(),
            notify_drop: None,
        }
    }
}

#[derive(Clone)]
pub struct ActionSender<Action: Clone> {
    sender: flume::Sender<Action>,
    updater: Arc<dyn Fn() + Send + Sync + 'static>,
}

impl<Action: Clone> ActionSender<Action> {
    pub fn send(&self, action: Action) {
        self.sender.send(action);
        (*self.updater)();
    }
}

impl<'a, ParentR: Reducer> VviewStore<'a, ParentR> {
    // FIXME: Figure out the reset_state!
    pub fn host<ChildR: ChildReducer<ParentR, Environment = ParentR::Environment>, T>(
        &'a self,
        cx: Scope<'a, T>,
        // reducer: ChildR,
        // root: fn(Scope<'a, T>, store: VviewStore<'a, ChildR>) -> Element<'a>,
        //) -> Element<'a>
    ) -> VviewStore<'a, ChildR>
    where
        // 'a: 'b,
        ChildR: 'static,
        ParentR: 'static,
    {
        let environment = self.environment;
        let child_state = cx.use_hook(|| unsafe {
            //Cell::new(Some(ChildR::initial_state(&environment)))
            MaybeUninit::new(ChildR::initial_state(&environment))
        });

        // FIXME: reset_state
        // let reset_state = false;
        // if reset_state {
        //     *child_state.write_silent() = state();
        // }

        let (child_sender, child_receiver) = cx.use_hook(|| flume::unbounded());

        let scope_id = cx.scope_id().0;
        let mut parent_runtime = self.runtime.write_silent();
        let parent_sender = parent_runtime.sender.clone();

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
                cloned_child_sender.send(child_message);
                updater();
            });
            if parent_runtime.child_senders.contains_key(&scope_id) {
                println!("ERROR: Hosted two child reducers in the same scope");
                println!("{}", include_str!("error_message.txt"));
            }
            parent_runtime.child_senders.insert(scope_id, sender);
        });

        // FIXME: Communicate from a callback? (check where this is used)

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
        };

        // rrun(cx, context, child_state, external_events, reducer)

        // let child_view_store = run(context, reducer_state, environment_builder, initial_action, external_events, reducer)
        let view_store = rrun(
            cx,
            &mut context,
            child_state,
            environment,
            child_sender,
            None,
            // reducer,
        );

        // root(cx, view_store)
        view_store
    }
}

// pub trait HostChild

pub fn root<'a, R: Reducer, T>(
    cx: Scope<'a, T>,
    // reducer: R,
    environment: &'a R::Environment,
) -> VviewStore<'a, R>
where
    R: 'static,
{
    //let state = cx.use_hook(|| Cell::new(Some(R::initial_state(environment))));
    let state = cx.use_hook(|| MaybeUninit::new(R::initial_state(environment)));

    let (child_sender, action_receiver) = cx.use_hook(|| flume::unbounded());

    let scope_id = cx.scope_id().0;

    // FIXME: Root shouldn't have delegate_sender. move to child only
    let delegate_sender = cx.use_hook(|| move |action| {});

    let mut context: ReducerContext<'a, R::Action, R::Message, R::DelegateMessage> =
        ReducerContext {
            action_receiver,
            receivers: Default::default(),
            delegate_messages: &*delegate_sender,
            child_messages: Vec::new(),
            window: AppWindow::retrieve(&cx),
            timers: Default::default(),
        };

    let view_store = rrun(
        cx,
        &mut context,
        state,
        environment,
        child_sender,
        None,
        // reducer,
    );

    view_store
}

// FIXME: Reducers always send `Actions` which are then converted to `DelegateMessage` if they
// come from a Child or to `Message` if they come from a parent
pub struct ReducerContext<'a, Action, Message, DelegateMessage> {
    /// The queue of next actions to this reducer
    action_receiver: &'a flume::Receiver<Action>,
    /// Additional receivers from different sources, such as subscriptions
    receivers: FxHashMap<u64, flume::Receiver<Action>>,
    /// Delegate messages to the parent
    delegate_messages: &'a dyn Fn(DelegateMessage),
    /// Send messages to the child reducers
    child_messages: Vec<Rc<dyn Fn(Message)>>,
    /// Allow accessing the current window without `use_window`
    window: AppWindow<'a>,
    /// Currently running timers
    timers: FxHashMap<AnyHashable, tokio::task::JoinHandle<()>>,
    // The reducer
}

impl<'a, Action, Message: Clone, DelegateMessage>
    ReducerContext<'a, Action, Message, DelegateMessage>
{
    pub fn send_parent(&self, message: DelegateMessage) {
        (self.delegate_messages)(message);
    }

    pub fn send_children(&self, message: Message) {
        self.child_messages
            .iter()
            .map(|child| child(message.clone()));
    }
}

fn rrun<'a, T, R: Reducer + 'static>(
    cx: Scope<'a, T>,
    context: &mut ReducerContext<'a, R::Action, R::Message, R::DelegateMessage>,
    //state: &'a mut Cell<Option<R::State>>,
    state: &'a mut MaybeUninit<R::State>,
    environment: &'a R::Environment,
    action_sender: &'a mut flume::Sender<R::Action>,
    external_events: Option<Vec<Box<dyn IntoMessageSender<R::Message>>>>,
    // reducer: R,
) -> VviewStore<'a, R>
// where
//     R: 'static,
{
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

    let external_receiver = cx.use_hook(|| {
        if let Some(senders) = external_events {
            let (external_sender, external_receiver) = flume::unbounded::<R::Action>();
            let cloned_updater = updater.clone();
            let wrapped_sender = Arc::new(move |message: R::Message| {
                external_sender.send(message.into_action());
                cloned_updater();
            });

            let mut output = Vec::with_capacity(senders.len() + 1);
            for sender in senders {
                output.push(sender.into_sender(wrapped_sender.clone()))
            }
            Some(external_receiver)
        } else {
            None
        }
    });

    let runtime: &UseRef<Rruntime<R>> = use_ref(cx, || {
        Rruntime::new(ActionSender {
            sender: action_sender.clone(),
            updater: updater.clone(),
        })
    });

    // the child senders are created later in the runtime (as the user interacts),
    // but once this code is called, they exist. so we can clone thme into the
    // parent so that they can be executed
    let current_child_senders = &runtime.read().child_senders;
    if !current_child_senders.is_empty() {
        // Need to be wrapped so they convert from Message to Action
        context.child_messages = current_child_senders
            .values()
            .cloned()
            // .map(|e| Rc::new(move |message| e(message.into_action())))
            .collect();
    }

    let mut known_actions = Vec::new();

    // Get the initial action
    if let Some(initial_action) = R::initial_action() {
        known_actions.push(initial_action);
    }

    // Read all events that have been sent
    for action in context.action_receiver.try_iter() {
        known_actions.push(action);
    }
    // Read all external events
    if let Some(ref receiver) = external_receiver {
        for action in receiver.try_iter() {
            known_actions.push(action);
        }
    }
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
                    cloned_sender.send(output);
                });
            }
        },
    );

    let eval = dioxus_desktop::use_eval(&cx);

    // Convert actions into effects and then handle in a loop
    let mut effects: Vec<_> = known_actions.drain(0..).map(Effect::Action).collect();

    if !effects.is_empty() {
        // let mut current_runtime = runtime.write_silent();

        let mut current_state = unsafe { state.assume_init_mut() };

        loop {
            println!("loooop");
            let mut additions: Vec<Effect<'_, R::Action>> = Vec::with_capacity(2);
            for effect in effects.drain(0..) {
                match effect {
                    Effect::Future(fut) => {
                        // coroutine.send(fut)
                        panic!()
                    }
                    Effect::Action(action) => {
                        let next =
                            R::reduce(&*context, action, &mut current_state, environment.deref());
                        additions.push(next);
                        continue;
                    }
                    Effect::Subscription(receiver, hash) => {
                        context.receivers.insert(hash.id(), receiver);
                        // runtime.with_mut(|r| r.receivers.push(Some(receiver)))
                    }
                    Effect::Multiple(mut v) => {
                        additions.append(&mut v);
                        continue;
                    }
                    Effect::Nothing => (),
                    Effect::Ui(s) => {
                        let eval = eval.clone();
                        cx.push_future(async move {
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
                        let cloned_sender = sender.clone();
                        cx.push_future(async move {
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
                                cloned_sender.send(n);
                            }
                        });
                    }
                    Effect::Timer(duration, action, id) => {
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
                    Effect::CancelTimer(id) => {
                        if let Some(n) = context.timers.remove(&id) {
                            n.abort();
                        }
                    }
                }
            }
            println!("additions {}", additions.len());
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

    println!("le done");

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

pub trait IntoAnyHashable {
    fn into_anyhashable(&self) -> AnyHashable;
}

impl<'a> IntoAnyHashable for &str {
    fn into_anyhashable(&self) -> AnyHashable {
        AnyHashable(self.hashed())
    }
}

impl<'a> IntoAnyHashable for usize {
    fn into_anyhashable(&self) -> AnyHashable {
        AnyHashable(self.hashed())
    }
}
