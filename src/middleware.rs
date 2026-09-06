use std::{
    marker::PhantomData,
    rc::{Rc, Weak},
};

#[cfg(feature = "thunk")]
use crate::middlewares::thunk::{ThunkCall, ThunkTask};
use crate::{DispatchError, DispatchResult, Selector, Store, store::Inner};

/// The accepted action types of a stack. Inferred by the builder.
pub trait InputSet {
    type Input;
}

/// The reducer's original action type.
pub struct Single<Action>(PhantomData<fn(Action)>);

impl<Action> InputSet for Single<Action> {
    type Input = Action;
}

/// A new input type plus all inputs accepted before it was introduced.
pub struct Extended<Action, Previous>(PhantomData<fn(Action, Previous)>);

impl<Action, Previous> InputSet for Extended<Action, Previous> {
    type Input = Action;
}

#[doc(hidden)]
pub struct Here;
#[doc(hidden)]
pub struct There<Path>(PhantomData<fn(Path)>);

/// Converts an accepted action through every intervening wrapper.
/// Use this bound in generic dispatch helpers; `Path` is inferred at the call site.
pub trait Promote<Action, Path>: InputSet {
    fn promote(action: Action) -> Self::Input;
}

impl<Action> Promote<Action, Here> for Single<Action> {
    fn promote(action: Action) -> Action {
        action
    }
}

impl<Action, Previous> Promote<Action, Here> for Extended<Action, Previous> {
    fn promote(action: Action) -> Action {
        action
    }
}

impl<Action, Previous, Input, Path> Promote<Input, There<Path>> for Extended<Action, Previous>
where
    Previous: Promote<Input, Path>,
    Action: From<Previous::Input>,
{
    fn promote(input: Input) -> Action {
        Previous::promote(input).into()
    }
}

/// Access to the complete chain. Cloning this API does not keep the store alive,
/// so middleware can retain it without creating an ownership cycle.
pub struct MiddlewareApi<State, Inputs: InputSet, Output> {
    pub(crate) inner: Weak<Inner<State, Inputs, Output>>,
}

impl<State, Inputs: InputSet, Output> Clone for MiddlewareApi<State, Inputs, Output> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<State, Inputs: InputSet, Output> MiddlewareApi<State, Inputs, Output> {
    pub(crate) fn store(&self) -> DispatchResult<Store<State, Inputs::Input, Output, Inputs>> {
        self.inner
            .upgrade()
            .map(Store::from_inner)
            .ok_or(DispatchError::StoreDropped)
    }

    /// Start again at the outermost middleware.
    pub fn dispatch<Action, Path>(
        &self,
        action: Action,
    ) -> <Action as DispatchInput<State, Inputs, Output, Path>>::Output
    where
        Action: DispatchInput<State, Inputs, Output, Path>,
    {
        action.dispatch_into(self.store().and_then(Store::for_dispatch))
    }

    pub fn try_select<S: Selector<State>>(&self, selector: S) -> DispatchResult<S::Result> {
        self.store()?.try_select(selector)
    }

    pub fn select<S: Selector<State>>(&self, selector: S) -> S::Result {
        self.try_select(selector).expect("cannot select state")
    }

    pub fn get_state(&self) -> State
    where
        State: Clone,
    {
        self.select(|state: &State| state.clone())
    }
}

/// The remaining middleware. Unlike [`MiddlewareApi::dispatch`], this does not
/// restart the chain. It may be called zero, one, or several times, or cloned
/// and retained for deferred forwarding. It does not keep the store alive.
pub struct Next<Action, Output> {
    pub(crate) dispatch: Rc<dyn Fn(Action) -> DispatchResult<Output>>,
    #[cfg(feature = "thunk")]
    pub(crate) thunk: Rc<dyn for<'a> Fn(ThunkCall<'a>) -> DispatchResult<ThunkTask<'a>>>,
}

impl<Action, Output> Clone for Next<Action, Output> {
    fn clone(&self) -> Self {
        Self {
            dispatch: self.dispatch.clone(),
            #[cfg(feature = "thunk")]
            thunk: self.thunk.clone(),
        }
    }
}

impl<Action, Output> Next<Action, Output> {
    pub fn dispatch(&self, action: impl Into<Action>) -> DispatchResult<Output> {
        (self.dispatch)(action.into())
    }

    /// Forward a thunk to the remaining layers without restarting the chain.
    #[cfg(feature = "thunk")]
    pub fn dispatch_thunk<'a>(&self, thunk: ThunkCall<'a>) -> DispatchResult<ThunkTask<'a>> {
        (self.thunk)(thunk)
    }
}

#[doc(hidden)]
pub struct Plain<Path>(PhantomData<fn(Path)>);

/// Typed dispatch overloads for ordinary actions and asynchronous thunks.
#[doc(hidden)]
pub trait DispatchInput<State, Inputs: InputSet, Output, Path> {
    type Output;
    fn dispatch_into(
        self,
        store: DispatchResult<Store<State, Inputs::Input, Output, Inputs>>,
    ) -> Self::Output;
}

impl<State, Inputs, Output, Action, Path> DispatchInput<State, Inputs, Output, Plain<Path>>
    for Action
where
    Inputs: Promote<Action, Path>,
{
    type Output = DispatchResult<Output>;
    fn dispatch_into(
        self,
        store: DispatchResult<Store<State, Inputs::Input, Output, Inputs>>,
    ) -> Self::Output {
        store?.dispatch_input(Inputs::promote(self))
    }
}

/// A layer with separately typed incoming actions, forwarded actions, and results.
/// Prefer [`middleware`] or [`crate::StoreBuilder::middleware`] for closures.
pub trait Middleware<State, Inner: InputSet, InnerOutput, Root: InputSet, RootOutput> {
    type Inputs: InputSet;
    type Output;

    fn dispatch(
        &self,
        api: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, InnerOutput>,
        action: <Self::Inputs as InputSet>::Input,
    ) -> DispatchResult<Self::Output>;

    /// Intercept a thunk at this layer's position in the chain. By default,
    /// forward unchanged. Only layers outside `ThunkMiddleware` see thunk calls;
    /// emitted actions still restart at the outermost layer.
    ///
    /// Return an error to reject the call, or forward and wrap the returned task
    /// to observe asynchronous completion. The caller retains its typed result.
    #[cfg(feature = "thunk")]
    fn dispatch_thunk<'a>(
        &self,
        _api: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, InnerOutput>,
        thunk: ThunkCall<'a>,
    ) -> DispatchResult<ThunkTask<'a>> {
        next.dispatch_thunk(thunk)
    }
}

/// A closure introducing a new action type. For an unchanged input type, use the
/// builder's `.middleware()` method instead.
pub struct MiddlewareFn<F, Input, Output> {
    handler: F,
    types: PhantomData<fn(Input) -> Output>,
}

pub fn middleware<State, InnerInput, InnerOutput, Root: InputSet, RootOutput, Input, Output, F>(
    handler: F,
) -> MiddlewareFn<F, Input, Output>
where
    F: Fn(
        &MiddlewareApi<State, Root, RootOutput>,
        Next<InnerInput, InnerOutput>,
        Input,
    ) -> DispatchResult<Output>,
{
    MiddlewareFn {
        handler,
        types: PhantomData,
    }
}

impl<State, Inner, InnerOutput, Root, RootOutput, Input, Output, F>
    Middleware<State, Inner, InnerOutput, Root, RootOutput> for MiddlewareFn<F, Input, Output>
where
    Inner: InputSet,
    Root: InputSet,
    Input: From<Inner::Input>,
    F: Fn(
        &MiddlewareApi<State, Root, RootOutput>,
        Next<Inner::Input, InnerOutput>,
        Input,
    ) -> DispatchResult<Output>,
{
    type Inputs = Extended<Input, Inner>;
    type Output = Output;

    fn dispatch(
        &self,
        api: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, InnerOutput>,
        action: Input,
    ) -> DispatchResult<Output> {
        (self.handler)(api, next, action)
    }
}

pub(crate) struct Preserve<F, Output>(pub F, pub PhantomData<fn() -> Output>);

impl<State, Inner, InnerOutput, Root, RootOutput, Output, F>
    Middleware<State, Inner, InnerOutput, Root, RootOutput> for Preserve<F, Output>
where
    Inner: InputSet,
    Root: InputSet,
    F: Fn(
        &MiddlewareApi<State, Root, RootOutput>,
        Next<Inner::Input, InnerOutput>,
        Inner::Input,
    ) -> DispatchResult<Output>,
{
    type Inputs = Inner;
    type Output = Output;

    fn dispatch(
        &self,
        api: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, InnerOutput>,
        action: Inner::Input,
    ) -> DispatchResult<Output> {
        (self.0)(api, next, action)
    }
}
