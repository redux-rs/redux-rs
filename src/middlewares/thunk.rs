use std::{
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use crate::{
    DispatchError, DispatchResult, Extended, InputSet, Middleware, MiddlewareApi, Next, Store,
    middleware::{DispatchInput, Promote},
};

/// Handles thunk calls at this position in the middleware chain. Outer layers
/// can intercept calls; inner layers see only the actions emitted by thunks.
/// Emitted actions always restart at the outermost layer.
pub struct ThunkMiddleware;

/// Completion visible to middleware without erasing the caller's result type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThunkOutcome {
    Succeeded,
    Failed,
}

/// Middleware may wrap this future to delay polling, observe completion, or
/// return a `DispatchError`. The caller's application value/error stays typed.
/// A wrapper must await the original task to preserve that result; returning
/// success without running it is reported as `DispatchError::Rejected`.
pub type ThunkTask<'a> = Pin<Box<dyn Future<Output = DispatchResult<ThunkOutcome>> + 'a>>;

/// A single-use thunk awaiting execution by `ThunkMiddleware`.
/// Forward with `Next::dispatch_thunk`, or return an error to reject it before
/// even its future factory runs. Captures may borrow caller-owned data.
pub struct ThunkCall<'a>(Box<dyn FnOnce() -> ThunkTask<'a> + 'a>);

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

    fn dispatch_thunk<'a>(
        &self,
        _: &MiddlewareApi<State, Root, RootOutput>,
        _: Next<Inner::Input, Output>,
        thunk: ThunkCall<'a>,
    ) -> DispatchResult<ThunkTask<'a>> {
        Ok((thunk.0)())
    }
}

pub(crate) struct ThunkInterceptor<F>(pub F);

impl<State, Inner: InputSet, Output, Root: InputSet, RootOutput, F>
    Middleware<State, Inner, Output, Root, RootOutput> for ThunkInterceptor<F>
where
    F: for<'a> Fn(
        &MiddlewareApi<State, Root, RootOutput>,
        Next<Inner::Input, Output>,
        ThunkCall<'a>,
    ) -> DispatchResult<ThunkTask<'a>>,
{
    type Inputs = Inner;
    type Output = Output;

    fn dispatch(
        &self,
        _: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, Output>,
        action: Inner::Input,
    ) -> DispatchResult<Output> {
        next.dispatch(action)
    }

    fn dispatch_thunk<'a>(
        &self,
        api: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, Output>,
        thunk: ThunkCall<'a>,
    ) -> DispatchResult<ThunkTask<'a>> {
        (self.0)(api, next, thunk)
    }
}

/// A consuming asynchronous closure, with no `Send` or `'static` requirement.
/// Its captures, future, and result may borrow caller-owned data.
pub struct Thunk<'a, F>(F, PhantomData<&'a ()>);

pub fn thunk<'a, State, Inputs: InputSet, Output, F, Fut>(f: F) -> Thunk<'a, F>
where
    F: FnOnce(MiddlewareApi<State, Inputs, Output>) -> Fut + 'a,
    Fut: Future,
{
    Thunk(f, PhantomData)
}

#[doc(hidden)]
pub struct ThunkPath;

/// An awaitable thunk result. Keeps the store alive until completion or cancellation.
#[must_use = "the thunk's asynchronous body runs when this future is polled"]
pub struct ThunkFuture<'a, State, Inputs: InputSet, Output, Value, Error> {
    future: Option<ThunkTask<'a>>,
    store: Option<Store<State, Inputs::Input, Output, Inputs>>,
    result: Rc<RefCell<Option<Result<Value, Error>>>>,
    error: Option<DispatchError>,
}

impl<'a, State, Inputs, Output, F, Fut, Value, Error>
    DispatchInput<State, Inputs, Output, ThunkPath> for Thunk<'a, F>
where
    State: 'a,
    Inputs: AllowsThunks + 'a,
    Output: 'a,
    F: FnOnce(MiddlewareApi<State, Inputs, Output>) -> Fut + 'a,
    Fut: Future<Output = Result<Value, Error>> + 'a,
    Value: 'a,
    Error: From<DispatchError> + 'a,
{
    type Output = ThunkFuture<'a, State, Inputs, Output, Value, Error>;

    fn dispatch_into(
        self,
        store: DispatchResult<Store<State, Inputs::Input, Output, Inputs>>,
    ) -> Self::Output {
        let mut dispatched = ThunkFuture {
            future: None,
            store: None,
            result: Rc::new(RefCell::new(None)),
            error: None,
        };
        let dispatch = store.and_then(|store| {
            let api = store.api();
            let result = dispatched.result.clone();
            let call = ThunkCall(Box::new(move || {
                let future = (self.0)(api);
                Box::pin(async move {
                    let value = future.await;
                    let outcome = if value.is_ok() {
                        ThunkOutcome::Succeeded
                    } else {
                        ThunkOutcome::Failed
                    };
                    // Only the completion status crosses the middleware boundary.
                    // Keep the application value/error, including borrows, intact.
                    *result.borrow_mut() = Some(value);
                    Ok(outcome)
                })
            }));
            let future = store.dispatch_thunk(call)?;
            dispatched.future = Some(future);
            dispatched.store = Some(store);
            Ok(())
        });
        if let Err(error) = dispatch {
            dispatched.error = Some(error);
            // An interceptor may have completed the task before rejecting it.
            drop(dispatched.result.replace(None));
        }
        dispatched
    }
}

impl<State, Inputs: InputSet, Output, Value, Error> Future
    for ThunkFuture<'_, State, Inputs, Output, Value, Error>
where
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
        let Poll::Ready(outcome) = result else {
            return Poll::Pending;
        };
        this.future = None;
        let value = this.result.borrow_mut().take();
        this.store = None;
        Poll::Ready(match outcome {
            Err(error) => Err(error.into()),
            Ok(_) => value.unwrap_or_else(|| {
                Err(
                    DispatchError::Rejected("thunk middleware completed without a result".into())
                        .into(),
                )
            }),
        })
    }
}
