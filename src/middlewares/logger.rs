use crate::{MiddleWare, StoreApi};
use async_trait::async_trait;
use log::{log, Level};
use std::fmt::Debug;
use std::sync::Arc;

pub struct LoggerMiddleware {
    log_level: Level,
}

impl LoggerMiddleware {
    pub fn new(log_level: Level) -> Self {
        LoggerMiddleware { log_level }
    }
}

#[async_trait]
impl<State, Action> MiddleWare<State, Action> for LoggerMiddleware
where
    State: Send + 'static,
    Action: Debug + Send + 'static,
{
    async fn dispatch<Inner>(&self, action: Action, inner: &Arc<Inner>)
    where
        Inner: StoreApi<State, Action> + Send + Sync,
    {
        log!(self.log_level, "Action: {:?}", action);
        inner.dispatch(action).await
    }
}
