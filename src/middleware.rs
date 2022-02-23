use crate::{Selector, Subscriber};
use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::Arc;

#[async_trait]
pub trait StoreApi<State, Action>
where
    Action: Send + 'static,
    State: Send + 'static
{
    async fn dispatch(&self, action: Action);
    async fn select<S: Selector<State, Result = Result>, Result>(&self, selector: S) -> Result
    where
        S: Selector<State, Result = Result> + Send + 'static,
        Result: Send + 'static;

    async fn state_cloned(&self) -> State
    where
        State: Clone
    {
        self.select(|state: &State| state.clone()).await
    }

    async fn subscribe<S: Subscriber<State> + Send + 'static>(&self, subscriber: S);
}

#[async_trait]
pub trait MiddleWare<State, Action, InnerAction>
where
    Action: Send + 'static,
    State: Send + 'static,
    InnerAction: Send + 'static
{
    #[allow(unused_variables)]
    async fn init<Inner>(&mut self, inner: &Arc<Inner>)
    where
        Inner: StoreApi<State, InnerAction> + Send + Sync
    {
    }

    #[allow(unused_variables)]
    async fn dispatch<Inner>(&self, action: Action, inner: &Arc<Inner>)
    where
        Inner: StoreApi<State, InnerAction> + Send + Sync
    {
    }
}

pub struct StoreWithMiddleware<Inner, M, State, InnerAction, OuterAction>
where
    Inner: StoreApi<State, InnerAction> + Send + Sync,
    M: MiddleWare<State, OuterAction, InnerAction> + Send + Sync,
    State: Send + Sync + 'static,
    InnerAction: Send + Sync + 'static,
    OuterAction: Send + Sync + 'static
{
    inner: Arc<Inner>,
    middleware: M,

    _types: PhantomData<(State, InnerAction, OuterAction)>
}

impl<Inner, M, State, InnerAction, OuterAction>
    StoreWithMiddleware<Inner, M, State, InnerAction, OuterAction>
where
    Inner: StoreApi<State, InnerAction> + Send + Sync,
    M: MiddleWare<State, OuterAction, InnerAction> + Send + Sync,
    State: Send + Sync + 'static,
    InnerAction: Send + Sync + 'static,
    OuterAction: Send + Sync + 'static
{
    pub async fn new(inner: Inner, mut middleware: M) -> Self {
        let inner = Arc::new(inner);

        middleware.init(&inner).await;

        StoreWithMiddleware {
            inner,
            middleware,
            _types: Default::default()
        }
    }

    pub async fn wrap<MNew, NewOuterAction>(
        self,
        middleware: MNew
    ) -> StoreWithMiddleware<Self, MNew, State, OuterAction, NewOuterAction>
    where
        MNew: MiddleWare<State, NewOuterAction, OuterAction> + Send + Sync,
        NewOuterAction: Send + Sync + 'static,
        State: Sync
    {
        StoreWithMiddleware::new(self, middleware).await
    }
}

#[async_trait]
impl<Inner, M, State, InnerAction, OuterAction> StoreApi<State, OuterAction>
    for StoreWithMiddleware<Inner, M, State, InnerAction, OuterAction>
where
    Inner: StoreApi<State, InnerAction> + Send + Sync,
    M: MiddleWare<State, OuterAction, InnerAction> + Send + Sync,
    State: Send + Sync + 'static,
    InnerAction: Send + Sync + 'static,
    OuterAction: Send + Sync + 'static
{
    async fn dispatch(&self, action: OuterAction) {
        self.middleware.dispatch(action, &self.inner).await
    }

    async fn select<S: Selector<State, Result = Result>, Result>(&self, selector: S) -> Result
    where
        S: Selector<State, Result = Result> + Send + 'static,
        Result: Send + 'static
    {
        self.inner.select(selector).await
    }

    async fn subscribe<S: Subscriber<State> + Send + 'static>(&self, subscriber: S) {
        self.inner.subscribe(subscriber).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use std::sync::Mutex;

    #[derive(Default)]
    struct LogStore {
        logs: Vec<String>
    }

    struct Log(String);

    fn log_reducer(store: LogStore, action: Log) -> LogStore {
        let mut logs = store.logs;
        logs.push(action.0);

        LogStore { logs }
    }

    struct LoggerMiddleware {
        prefix: &'static str,
        logs: Arc<Mutex<Vec<String>>>
    }

    impl LoggerMiddleware {
        pub fn new(prefix: &'static str, logs: Arc<Mutex<Vec<String>>>) -> Self {
            LoggerMiddleware { logs, prefix }
        }

        pub fn log(&self, message: String) {
            let mut logs = self.logs.lock().unwrap();
            logs.push(format!("[{}] {}", self.prefix, message));
        }
    }

    #[async_trait]
    impl MiddleWare<LogStore, Log, Log> for LoggerMiddleware {
        async fn dispatch<Inner>(&self, action: Log, inner: &Arc<Inner>)
        where
            Inner: StoreApi<LogStore, Log> + Send + Sync
        {
            let log_message = action.0.clone();

            // Simulate logging to the console, we log to a vec so we can unit test
            self.log(format!("Before dispatching log message: {:?}", log_message));

            // Dispatch the actual action
            inner.dispatch(action).await;

            // Simulate logging to the console, we log to a vec so we can unit test
            self.log(format!("After dispatching log message: {:?}", log_message));
        }
    }

    #[tokio::test]
    async fn logger_middleware() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let log_middleware = LoggerMiddleware::new("log", logs.clone());

        let store = Store::new(log_reducer).wrap(log_middleware).await;

        store.dispatch(Log("Log 1".to_string())).await;

        {
            let lock = logs.lock().unwrap();
            let logs: &Vec<String> = lock.as_ref();
            assert_eq!(
                logs,
                &vec![
                    "[log] Before dispatching log message: \"Log 1\"".to_string(),
                    "[log] After dispatching log message: \"Log 1\"".to_string(),
                ]
            );
        }

        store.dispatch(Log("Log 2".to_string())).await;

        {
            let lock = logs.lock().unwrap();
            let logs: &Vec<String> = lock.as_ref();
            assert_eq!(
                logs,
                &vec![
                    "[log] Before dispatching log message: \"Log 1\"".to_string(),
                    "[log] After dispatching log message: \"Log 1\"".to_string(),
                    "[log] Before dispatching log message: \"Log 2\"".to_string(),
                    "[log] After dispatching log message: \"Log 2\"".to_string()
                ]
            );
        }
    }

    #[tokio::test]
    async fn logger_nested_middlewares() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let log_middleware_1 = LoggerMiddleware::new("middleware_1", logs.clone());
        let log_middleware_2 = LoggerMiddleware::new("middleware_2", logs.clone());

        let store = Store::new(log_reducer)
            .wrap(log_middleware_1)
            .await
            .wrap(log_middleware_2)
            .await;

        store.dispatch(Log("Log 1".to_string())).await;

        {
            let lock = logs.lock().unwrap();
            let logs: &Vec<String> = lock.as_ref();
            assert_eq!(
                logs,
                &vec![
                    "[middleware_2] Before dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_1] Before dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_1] After dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_2] After dispatching log message: \"Log 1\"".to_string(),
                ]
            );
        }

        store.dispatch(Log("Log 2".to_string())).await;

        {
            let lock = logs.lock().unwrap();
            let logs: &Vec<String> = lock.as_ref();
            assert_eq!(
                logs,
                &vec![
                    "[middleware_2] Before dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_1] Before dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_1] After dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_2] After dispatching log message: \"Log 1\"".to_string(),
                    "[middleware_2] Before dispatching log message: \"Log 2\"".to_string(),
                    "[middleware_1] Before dispatching log message: \"Log 2\"".to_string(),
                    "[middleware_1] After dispatching log message: \"Log 2\"".to_string(),
                    "[middleware_2] After dispatching log message: \"Log 2\"".to_string(),
                ]
            );
        }
    }
}
