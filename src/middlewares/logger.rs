use crate::{DispatchResult, InputSet, Middleware, MiddlewareApi, Next};
use log::{Level, log};
use std::fmt::Debug;

/// Logs actions at a configurable level using the application's `log` subscriber.
/// With the `thunk` feature, also logs thunk calls when outside `ThunkMiddleware`.
pub struct LoggerMiddleware {
    level: Level,
}

impl LoggerMiddleware {
    pub fn new(level: Level) -> Self {
        Self { level }
    }
}

impl<State, Inner, Output, Root, RootOutput> Middleware<State, Inner, Output, Root, RootOutput>
    for LoggerMiddleware
where
    Inner: InputSet,
    Inner::Input: Debug,
    Root: InputSet,
{
    type Inputs = Inner;
    type Output = Output;

    fn dispatch(
        &self,
        _: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, Output>,
        action: Inner::Input,
    ) -> DispatchResult<Output> {
        log!(self.level, "Action: {action:?}");
        next.dispatch(action)
    }

    #[cfg(feature = "thunk")]
    fn dispatch_thunk<'a>(
        &self,
        _: &MiddlewareApi<State, Root, RootOutput>,
        next: Next<Inner::Input, Output>,
        thunk: crate::middlewares::thunk::ThunkCall<'a>,
    ) -> DispatchResult<crate::middlewares::thunk::ThunkTask<'a>> {
        log!(self.level, "Thunk dispatched");
        next.dispatch_thunk(thunk)
    }
}
