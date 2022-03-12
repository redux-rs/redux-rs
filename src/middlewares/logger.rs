use crate::{MiddleWare, StoreApi};
use async_trait::async_trait;
use log::{log, Level};
use std::fmt::Debug;
use std::sync::Arc;

/// A middleware which logs every single action that has been dispatched to the store
/// We're using the `log` crate to achieve the logging, the log level can be set
///
/// ## Usage:
/// ```
/// # #[derive(Default)]
/// # struct EmptyStore;
/// #
/// # #[derive(Debug)]
/// # struct LogableAction(&'static str);
/// #
/// # fn reducer(store: EmptyStore, _action: LogableAction) -> EmptyStore {
/// #     store
/// # }
/// use log::Level;
/// use redux_rs::{
///     middlewares::logger::LoggerMiddleware,
///     Store
/// };
/// # async fn async_test() {
/// // Setup the logger middleware with default "Debug" log level
/// let logger_middleware = LoggerMiddleware::new(Level::Debug);
///
/// // Create a new store and wrap it with the logger middleware
/// let store = Store::new(reducer).wrap(logger_middleware).await;
/// # }
/// ```
pub struct LoggerMiddleware {
    log_level: Level,
}

impl LoggerMiddleware {
    /// Crate a new logger.
    /// LogLevel is the level that the logs will be output with
    pub fn new(log_level: Level) -> Self {
        LoggerMiddleware { log_level }
    }
}

#[async_trait]
impl<State, Action, Inner> MiddleWare<State, Action, Inner> for LoggerMiddleware
where
    State: Send + 'static,
    Action: Debug + Send + 'static,
    Inner: StoreApi<State, Action> + Send + Sync,
{
    async fn dispatch(&self, action: Action, inner: &Arc<Inner>) {
        // Log the action
        log!(self.log_level, "Action: {:?}", action);

        // Continue dispatching the action
        inner.dispatch(action).await
    }
}
