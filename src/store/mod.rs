use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    marker::PhantomData,
    rc::Rc,
};

use crate::{
    DispatchError, DispatchResult, InputSet, Middleware, MiddlewareApi, Next, Reducer, Selector,
    Single, Subscriber, Subscription,
    middleware::{DispatchInput, Preserve},
};

#[cfg(feature = "thunk")]
use crate::middlewares::thunk::{ThunkCall, ThunkInterceptor, ThunkTask};

type BuildHandler<State, Input, Output, Root, RootOutput> =
    Box<dyn FnOnce(MiddlewareApi<State, Root, RootOutput>) -> Next<Input, Output>>;
type Listener = Rc<dyn Fn()>;

pub(crate) struct Core<State> {
    state: RefCell<Option<State>>,
    reducing: Cell<bool>,
    listeners: RefCell<BTreeMap<u64, Listener>>,
    next_listener: Cell<u64>,
}

impl<State> Core<State> {
    fn reduce<Action>(
        &self,
        reducer: &impl Reducer<State, Action>,
        action: &Action,
    ) -> DispatchResult<()> {
        if self.reducing.get() {
            return Err(DispatchError::Reducing);
        }
        let state = self
            .state
            .try_borrow_mut()
            .map_err(|_| DispatchError::StateBorrowed)?
            .take()
            .ok_or(DispatchError::Poisoned)?;
        {
            // Reset even if a reducer unwinds. Missing state then marks the store poisoned.
            struct Reducing<'a>(&'a Cell<bool>);
            impl Drop for Reducing<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }
            self.reducing.set(true);
            let _reducing = Reducing(&self.reducing);
            let state = reducer.reduce(state, action);
            self.state.replace(Some(state));
        }
        let listeners: Vec<_> = self.listeners.borrow().values().cloned().collect();
        // No state or listener borrow survives a callback. Nested dispatch is legal.
        for listener in listeners {
            listener();
        }
        Ok(())
    }

    fn select<S: Selector<State>>(&self, selector: S) -> DispatchResult<S::Result> {
        if self.reducing.get() {
            return Err(DispatchError::Reducing);
        }
        let state = self
            .state
            .try_borrow()
            .map_err(|_| DispatchError::StateBorrowed)?;
        Ok(selector.select(state.as_ref().ok_or(DispatchError::Poisoned)?))
    }
}

pub(crate) struct Inner<State, Inputs: InputSet, Output> {
    core: Rc<Core<State>>,
    handler: Next<Inputs::Input, Output>,
}

/// A synchronous, cheaply cloneable store. No runtime, `Send`, or `Sync` required.
/// Clones share state and middleware. The final clone releases the state;
/// retained `Next` handles can keep downstream middleware/reducer captures alive.
///
/// `Action` is the outermost input; `Accepted` records earlier input types and is
/// inferred when building a wrapped store. Ordinary stores are `Store<State, Action>`.
pub struct Store<
    State,
    Action,
    Output = Action,
    Accepted: InputSet<Input = Action> = Single<Action>,
> {
    pub(crate) inner: Rc<Inner<State, Accepted, Output>>,
}

impl<State, Action, Output, Accepted: InputSet<Input = Action>> Clone
    for Store<State, Action, Output, Accepted>
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<State, Action, Output, Accepted: InputSet<Input = Action>>
    Store<State, Action, Output, Accepted>
{
    pub(crate) fn from_inner(inner: Rc<Inner<State, Accepted, Output>>) -> Self {
        Self { inner }
    }

    /// A weak handle suitable for callbacks or middleware that may outlive this store.
    pub fn api(&self) -> MiddlewareApi<State, Accepted, Output> {
        MiddlewareApi {
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Dispatch an accepted action or (with thunk middleware) an asynchronous thunk.
    /// Actions forwarded synchronously complete reduction and notification before
    /// this returns. Middleware may instead reject, swallow, or defer an action;
    /// a successful dispatch alone does not guarantee that reduction occurred.
    pub fn dispatch<Input, Path>(
        &self,
        input: Input,
    ) -> <Input as DispatchInput<State, Accepted, Output, Path>>::Output
    where
        Input: DispatchInput<State, Accepted, Output, Path>,
    {
        input.dispatch_into(self.clone().for_dispatch())
    }

    pub(crate) fn for_dispatch(self) -> DispatchResult<Self> {
        if self.inner.core.reducing.get() {
            return Err(DispatchError::Reducing);
        }
        Ok(self)
    }

    pub(crate) fn dispatch_input(&self, action: Action) -> DispatchResult<Output> {
        // Guard the entire chain, not just the final reducer.
        if self.inner.core.reducing.get() {
            return Err(DispatchError::Reducing);
        }
        self.inner.handler.dispatch(action)
    }

    #[cfg(feature = "thunk")]
    pub(crate) fn dispatch_thunk<'a>(&self, thunk: ThunkCall<'a>) -> DispatchResult<ThunkTask<'a>> {
        self.inner.handler.dispatch_thunk(thunk)
    }

    pub fn try_select<S: Selector<State>>(&self, selector: S) -> DispatchResult<S::Result> {
        self.inner.core.select(selector)
    }

    /// Read state without cloning it. The closure can borrow local variables or
    /// consume captures. Reduction during the selector returns `StateBorrowed`;
    /// middleware runs first and may swallow or defer the action successfully.
    pub fn select<S: Selector<State>>(&self, selector: S) -> S::Result {
        self.try_select(selector).expect("cannot select state")
    }

    /// Return an owned snapshot. Prefer `select` when only a portion is needed.
    pub fn get_state(&self) -> State
    where
        State: Clone,
    {
        self.select(|state: &State| state.clone())
    }

    pub fn state_cloned(&self) -> State
    where
        State: Clone,
    {
        self.get_state()
    }
}

impl<State: 'static, Action, Output, Accepted: InputSet<Input = Action>>
    Store<State, Action, Output, Accepted>
{
    /// Redux-style notification, with no snapshot allocation or `State: Clone` bound.
    /// Listeners can read, dispatch, subscribe, and unsubscribe. The listener list
    /// is snapshotted per dispatch; changes apply to subsequent (including nested) dispatches.
    pub fn listen(&self, listener: impl Fn() + 'static) -> Subscription {
        let core = &self.inner.core;
        assert!(!core.reducing.get(), "cannot subscribe inside a reducer");
        let id = core.next_listener.get();
        core.next_listener
            .set(id.checked_add(1).expect("subscription IDs exhausted"));
        core.listeners.borrow_mut().insert(id, Rc::new(listener));
        let weak = Rc::downgrade(core);
        Subscription::new(move || {
            if let Some(core) = weak.upgrade() {
                let listener = core.listeners.borrow_mut().remove(&id);
                drop(listener);
            }
        })
    }

    /// Convenience notification with an owned state snapshot. The snapshot keeps
    /// `&State` valid if the subscriber performs a nested dispatch. Use `listen`
    /// to avoid cloning large or non-Clone state.
    pub fn subscribe(&self, subscriber: impl Subscriber<State> + 'static) -> Subscription
    where
        State: Clone,
    {
        let weak = Rc::downgrade(&self.inner.core);
        self.listen(move || {
            if let Some(core) = weak.upgrade() {
                // ponytail: one clone per subscriber; use listen/select for large state.
                let snapshot = core
                    .select(|state: &State| state.clone())
                    .expect("cannot notify subscriber");
                subscriber.notify(&snapshot);
            }
        })
    }
}

impl<State: 'static, Action: 'static> Store<State, Action> {
    pub fn new(reducer: impl Reducer<State, Action> + 'static) -> Self
    where
        State: Default,
    {
        Self::builder(reducer).build()
    }

    pub fn new_with_state(reducer: impl Reducer<State, Action> + 'static, state: State) -> Self {
        Self::builder_with_state(reducer, state).build()
    }

    pub fn builder<Root: InputSet + 'static, RootOutput: 'static>(
        reducer: impl Reducer<State, Action> + 'static,
    ) -> StoreBuilder<State, Single<Action>, Action, Root, RootOutput>
    where
        State: Default,
    {
        Self::builder_with_state(reducer, State::default())
    }

    /// Builder for states without `Default`.
    pub fn builder_with_state<Root: InputSet + 'static, RootOutput: 'static>(
        reducer: impl Reducer<State, Action> + 'static,
        state: State,
    ) -> StoreBuilder<State, Single<Action>, Action, Root, RootOutput> {
        StoreBuilder {
            state,
            handler: Box::new(move |api| Next {
                dispatch: Rc::new(move |action| {
                    api.store()?.inner.core.reduce(&reducer, &action)?;
                    Ok(action)
                }),
                #[cfg(feature = "thunk")]
                thunk: Rc::new(|_| {
                    Err(DispatchError::Rejected(
                        "no thunk middleware handled the call".into(),
                    ))
                }),
            }),
        }
    }
}

/// Collects middleware before exposing a dispatcher. The last wrapper runs first.
/// The root API's types are inferred when `build` finalizes the complete chain.
pub struct StoreBuilder<State, Inputs: InputSet, Output, Root: InputSet, RootOutput> {
    state: State,
    handler: BuildHandler<State, Inputs::Input, Output, Root, RootOutput>,
}

impl<
    State: 'static,
    Inputs: InputSet + 'static,
    Output: 'static,
    Root: InputSet + 'static,
    RootOutput: 'static,
> StoreBuilder<State, Inputs, Output, Root, RootOutput>
where
    Inputs::Input: 'static,
{
    pub fn with_state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Add a layer. New input types are added using `middleware(...)` and a
    /// `From` implementation from the previous layer's input. Use distinct types
    /// for extensions; repeated input types belong in `.middleware(...)` instead.
    pub fn wrap<M>(
        self,
        middleware: M,
    ) -> StoreBuilder<State, M::Inputs, M::Output, Root, RootOutput>
    where
        M: Middleware<State, Inputs, Output, Root, RootOutput> + 'static,
        <M::Inputs as InputSet>::Input: 'static,
        M::Output: 'static,
    {
        StoreBuilder {
            state: self.state,
            handler: Box::new(move |api| {
                let next = (self.handler)(api.clone());
                #[cfg(feature = "thunk")]
                let middleware = Rc::new(middleware);
                #[cfg(feature = "thunk")]
                let thunk: Rc<
                    dyn for<'a> Fn(ThunkCall<'a>) -> DispatchResult<ThunkTask<'a>>,
                > = {
                    let (middleware, api, next) = (middleware.clone(), api.clone(), next.clone());
                    Rc::new(move |thunk| {
                        let _store = api.store()?.for_dispatch()?;
                        middleware.dispatch_thunk(&api, next.clone(), thunk)
                    })
                };
                Next {
                    dispatch: Rc::new(move |action| {
                        let _store = api.store()?.for_dispatch()?;
                        middleware.dispatch(&api, next.clone(), action)
                    }),
                    #[cfg(feature = "thunk")]
                    thunk,
                }
            }),
        }
    }

    /// Add a closure without changing accepted action types. Its result type can change.
    pub fn middleware<NewOutput: 'static, F>(
        self,
        handler: F,
    ) -> StoreBuilder<State, Inputs, NewOutput, Root, RootOutput>
    where
        F: Fn(
                &MiddlewareApi<State, Root, RootOutput>,
                Next<Inputs::Input, Output>,
                Inputs::Input,
            ) -> DispatchResult<NewOutput>
            + 'static,
    {
        self.wrap(Preserve(handler, PhantomData))
    }

    /// Add a thunk interceptor at this position; ordinary actions pass unchanged.
    /// Add it after `.wrap(ThunkMiddleware)` to observe calls before execution.
    /// Use `next.dispatch_thunk(call)` to forward, then optionally wrap its task.
    ///
    /// ```
    /// use redux_rs::{DispatchError, Store};
    /// use redux_rs::middlewares::thunk::{ThunkMiddleware, ThunkOutcome, thunk};
    /// # async fn example() -> Result<(), DispatchError> {
    /// let store = Store::builder(|state: i32, action: &i32| state + action)
    ///     .wrap(ThunkMiddleware)
    ///     .thunk_middleware(|api, next, call| {
    ///         if api.get_state() < 0 {
    ///             return Err(DispatchError::Rejected("thunks disabled".into()));
    ///         }
    ///         let future = next.dispatch_thunk(call)?;
    ///         Ok(Box::pin(async move {
    ///             let outcome = future.await?;
    ///             assert_eq!(outcome, ThunkOutcome::Succeeded);
    ///             Ok(outcome)
    ///         }))
    ///     })
    ///     .build();
    /// let value = store.dispatch(thunk(|api| async move {
    ///     api.dispatch(1_i32)?;
    ///     Ok::<_, DispatchError>(api.get_state())
    /// })).await?;
    /// assert_eq!(value, 1);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "thunk")]
    pub fn thunk_middleware<F>(self, handler: F) -> Self
    where
        F: for<'a> Fn(
                &MiddlewareApi<State, Root, RootOutput>,
                Next<Inputs::Input, Output>,
                ThunkCall<'a>,
            ) -> DispatchResult<ThunkTask<'a>>
            + 'static,
    {
        self.wrap(ThunkInterceptor(handler))
    }
}

impl<State, Inputs: InputSet, Output> StoreBuilder<State, Inputs, Output, Inputs, Output> {
    pub fn build(self) -> Store<State, Inputs::Input, Output, Inputs> {
        Store::from_inner(Rc::new_cyclic(|weak| Inner {
            core: Rc::new(Core {
                state: RefCell::new(Some(self.state)),
                reducing: Cell::new(false),
                listeners: RefCell::new(BTreeMap::new()),
                next_listener: Cell::new(0),
            }),
            handler: (self.handler)(MiddlewareApi {
                inner: weak.clone(),
            }),
        }))
    }
}
