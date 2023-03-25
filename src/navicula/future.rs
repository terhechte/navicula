use std::time::Duration;

use futures_util::{Future, FutureExt};

use super::effect::*;

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
