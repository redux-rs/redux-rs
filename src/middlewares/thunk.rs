use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    DispatchError, DispatchResult, Extended, InputSet, Middleware, MiddlewareApi, Next, Store,
    middleware::{DispatchInput, Promote},
};

/// Enables typed asynchronous function dispatch. Actions emitted by thunks enter
/// the complete action middleware chain. Function values themselves are handled
/// separately from action-only middleware closures.
pub struct ThunkMiddleware;

#[doc(hidden)]
pub struct WithThunks<Inputs>(PhantomData<fn(Inputs)>);

impl<Inputs: InputSet> InputSet for WithThunks<Inputs> {
    type Input = Inputs::Input;
}
impl<Inputs: Promote<Action, Path>, Action, Path> Promote<Action, Path> for WithThunks<Inputs> {
    fn promote(action: Action) -> Self::Input {
        Inputs::promote(action)
    }
}

#[doc(hidden)]
pub trait AllowsThunks: InputSet {}
impl<Inputs: InputSet> AllowsThunks for WithThunks<Inputs> {}
impl<Action, Previous: AllowsThunks> AllowsThunks for Extended<Action, Previous> {}

impl<State, Inner: InputSet, Output, Root: InputSet, RootOutput>
    Middleware<State, Inner, Output, Root, RootOutput> for ThunkMiddleware
{
    type Inputs = WithThunks<Inner>;
    type Output = Output;

    fn dispatch(
        &self,
        _: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, Output>,
        action: Inner::Input,
    ) -> DispatchResult<Output> {
        next.dispatch(action)
    }
}

/// A consuming asynchronous closure. No boxing, `Send`, or `'static` requirement
/// on the closure itself. Its future can borrow caller-owned data.
pub struct Thunk<F>(F);

pub fn thunk<State, Inputs: InputSet, Output, F, Fut>(f: F) -> Thunk<F>
where
    F: FnOnce(MiddlewareApi<State, Inputs, Output>) -> Fut,
    Fut: Future,
{
    Thunk(f)
}

#[doc(hidden)]
pub struct ThunkPath;

/// An awaitable thunk result. Keeps the store alive until completion or cancellation.
#[must_use = "the thunk's asynchronous body runs when this future is polled"]
pub struct ThunkFuture<State, Inputs: InputSet, Output, Fut> {
    future: Option<Pin<Box<Fut>>>,
    store: Option<Store<State, Inputs::Input, Output, Inputs>>,
    error: Option<DispatchError>,
}

impl<State, Inputs, Output, F, Fut, Value, Error> DispatchInput<State, Inputs, Output, ThunkPath>
    for Thunk<F>
where
    Inputs: AllowsThunks,
    F: FnOnce(MiddlewareApi<State, Inputs, Output>) -> Fut,
    Fut: Future<Output = Result<Value, Error>>,
    Error: From<DispatchError>,
{
    type Output = ThunkFuture<State, Inputs, Output, Fut>;

    fn dispatch_into(
        self,
        store: DispatchResult<Store<State, Inputs::Input, Output, Inputs>>,
    ) -> Self::Output {
        match store {
            Ok(store) => ThunkFuture {
                future: Some(Box::pin((self.0)(store.api()))),
                store: Some(store),
                error: None,
            },
            Err(error) => ThunkFuture {
                future: None,
                store: None,
                error: Some(error),
            },
        }
    }
}

impl<State, Inputs: InputSet, Output, Fut, Value, Error> Future
    for ThunkFuture<State, Inputs, Output, Fut>
where
    Fut: Future<Output = Result<Value, Error>>,
    Error: From<DispatchError>,
{
    type Output = Result<Value, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(error) = this.error.take() {
            return Poll::Ready(Err(error.into()));
        }
        let result = this
            .future
            .as_mut()
            .expect("thunk polled after completion")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            this.future = None;
            this.store = None;
        }
        result
    }
}
