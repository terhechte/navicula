use std::{pin::Pin, time::Duration};

use futures_util::{future::BoxFuture, Future};

use super::{
    publisher::{AnySubscription, Publisher, RefSubscription},
    traits::AnyHashable,
};

// FIXME: Wrap with InnerEffect so only methods expose effects
pub enum Effect<'a, A> {
    Future(BoxFuture<'a, A>),
    Action(A),
    // Subscription(Box<dyn EventReceiver<A>>),
    // Subscription(flume::Receiver<A>, AnyHashable),
    Subscription(AnySubscription),
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

    // pub fn subscribe(publisher: impl Publisher, id: impl IntoAnyHashable) -> Effect<'a, A> {
    //     Self::
    // }

    // pub fn timer(duration: Duration, action: A, identifier: impl Hash) -> Effect<'a, A> {
    //     let mut hasher = DefaultHasher::default();
    //     identifier.hash(&mut hasher);
    //     Effect::Timer(duration, action, hasher.finish())
    // }
}
