use std::{pin::Pin, time::Duration};

use futures_util::{future::BoxFuture, Future, FutureExt};

use super::{anyhashable::AnyHashable, publisher::AnySubscription};

pub(super) enum InnerEffect<'a, A> {
    Future(BoxFuture<'a, A>),
    FireForget(BoxFuture<'a, ()>),
    Action(A),
    Delay(Duration, Box<InnerEffect<'a, A>>),
    Subscription(AnySubscription),
    Multiple(Vec<InnerEffect<'a, A>>),
    Nothing,
    /// Execute the following javascript, ignore the result
    Ui(String),
    /// Execute the following javascript, but also get the result back and convert into an action
    UiFuture(Pin<Box<dyn Future<Output = Option<A>> + 'static>>),
    /// Maybe a better solution? Timer. the u64 parameter can be used to cancel it again
    Timer(Duration, A, AnyHashable),
    CancelTimer(AnyHashable),
}

pub struct Effect<'a, A>(InnerEffect<'a, A>);

impl<'a, A> Effect<'a, A> {
    pub const NONE: Self = Self(InnerEffect::Nothing);

    pub fn merge2(a: Effect<'a, A>, b: Effect<'a, A>) -> Self {
        Self(InnerEffect::Multiple(vec![a.0, b.0]))
    }

    pub fn merge3(a: Effect<'a, A>, b: Effect<'a, A>, c: Effect<'a, A>) -> Self {
        Self(InnerEffect::Multiple(vec![a.0, b.0, c.0]))
    }

    pub fn merge4(a: Effect<'a, A>, b: Effect<'a, A>, c: Effect<'a, A>, d: Effect<'a, A>) -> Self {
        Self(InnerEffect::Multiple(vec![a.0, b.0, c.0, d.0]))
    }

    pub fn merge5(
        a: Effect<'a, A>,
        b: Effect<'a, A>,
        c: Effect<'a, A>,
        d: Effect<'a, A>,
        e: Effect<'a, A>,
    ) -> Self {
        Self(InnerEffect::Multiple(vec![a.0, b.0, c.0, d.0, e.0]))
    }

    pub fn merge6(
        a: Effect<'a, A>,
        b: Effect<'a, A>,
        c: Effect<'a, A>,
        d: Effect<'a, A>,
        e: Effect<'a, A>,
        f: Effect<'a, A>,
    ) -> Self {
        Self(InnerEffect::Multiple(vec![a.0, b.0, c.0, d.0, e.0, f.0]))
    }

    pub fn future<T, FU, M>(fu: FU, mapper: M) -> Self
    where
        FU: Future<Output = T> + Send + 'a,
        M: FnOnce(T) -> A + Send + Sync + 'a,
    {
        Self(InnerEffect::Future(Box::pin(async move {
            fu.map(move |o| mapper(o)).await
        })))
    }

    pub fn fire_forget<FU>(fu: FU) -> Self
    where
        FU: Future<Output = ()> + Send + 'a,
    {
        Self(InnerEffect::FireForget(Box::pin(async move { fu.await })))
    }

    pub fn debounce<T, FU, M>(fu: FU, mapper: M, duration: Duration, bouncer: Debouncer) -> Self
    where
        FU: Future<Output = T> + Send + 'a,
        M: FnOnce(Option<T>) -> A + Send + Sync + 'a,
    {
        Self(InnerEffect::Future(Box::pin(async move {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(tokio::time::Duration::from_secs_f64(duration.as_secs_f64())).await;

            // if we were debounced, return
            let val = bouncer.0.load(std::sync::atomic::Ordering::Relaxed);
            if val {
                return mapper(None);
            }

            let o = fu.map(move |o| mapper(Some(o))).await;
            o
        })))
    }

    pub fn action(action: A) -> Self {
        Self(InnerEffect::Action(action))
    }

    pub fn subscription(s: AnySubscription) -> Self {
        Self(InnerEffect::Subscription(s))
    }

    pub fn nothing() -> Self {
        Self(InnerEffect::Nothing)
    }

    pub fn ui(s: impl AsRef<str>) -> Self {
        Self(InnerEffect::Ui(s.as_ref().to_string()))
    }

    pub fn ui_future(fut: impl Future<Output = Option<A>> + 'static) -> Self {
        Self(InnerEffect::UiFuture(Box::pin(fut)))
    }

    // FIXME: Maybe return a token that will cancel the timer on drop?
    pub fn timer(duration: Duration, action: A, hash: impl Into<AnyHashable>) -> Self {
        Self(InnerEffect::Timer(duration, action, hash.into()))
    }

    pub fn cancel_timer(hash: impl Into<AnyHashable>) -> Self {
        Self(InnerEffect::CancelTimer(hash.into()))
    }
}

impl<'a, A> Effect<'a, A> {
    pub fn delay(self, duration: Duration) -> Self {
        Self(InnerEffect::Delay(duration, Box::new(self.0)))
    }

    pub(super) fn inner(self) -> InnerEffect<'a, A> {
        self.0
    }
}

#[derive(Clone, Default, Debug)]
pub struct Debouncer(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Debouncer {
    pub fn cancel(&self) {
        self.0.swap(true, std::sync::atomic::Ordering::SeqCst);
    }
}
