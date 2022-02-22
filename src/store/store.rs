use super::worker::*;
use crate::{Reducer, Selector};
use std::marker::PhantomData;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

pub struct Store<State, Action, RootReducer> {
    sender: UnboundedSender<Work<State, Action>>,
    _worker_handle: JoinHandle<()>,

    _types: PhantomData<RootReducer>
}

impl<State, Action, RootReducer> Store<State, Action, RootReducer>
where
    Action: Send + 'static,
    RootReducer: Reducer<State, Action> + Send + 'static,
    State: Send + 'static
{
    pub fn new(root_reducer: RootReducer) -> Self
    where
        State: Default
    {
        Self::new_with_state(root_reducer, Default::default())
    }

    pub fn new_with_state(root_reducer: RootReducer, state: State) -> Self {
        let (mut worker, sender) = StateWorker::new(root_reducer, state);

        let _worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        Store {
            sender,
            _worker_handle,

            _types: Default::default()
        }
    }

    fn dispatch_work(&self, work: Work<State, Action>) {
        let _ = self.sender.send(work);
    }

    pub async fn dispatch(&self, action: Action) {
        let (work, notifier) = Work::reduce(action);
        self.dispatch_work(work);
        notifier.notified().await;
    }

    pub async fn select<S: Selector<State, Result = Result>, Result>(&self, selector: S) -> Result
    where
        S: Selector<State, Result = Result> + Send + 'static,
        Result: Send + 'static
    {
        let (work, result_receiver) = Work::select(selector);
        self.dispatch_work(work);
        return result_receiver.await.unwrap();
    }

    pub async fn state_cloned(&self) -> State
    where
        State: Clone
    {
        self.select(|state: &State| state.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Counter {
        value: i32
    }

    impl Counter {
        pub fn new(value: i32) -> Self {
            Counter { value }
        }
    }

    impl Default for Counter {
        fn default() -> Self {
            Self { value: 42 }
        }
    }

    struct ValueSelector;
    impl Selector<Counter> for ValueSelector {
        type Result = i32;

        fn select(&self, state: &Counter) -> Self::Result {
            state.value
        }
    }

    enum CounterAction {
        Increment,
        Decrement
    }

    fn counter_reducer(state: Counter, action: CounterAction) -> Counter {
        match action {
            CounterAction::Increment => Counter {
                value: state.value + 1
            },
            CounterAction::Decrement => Counter {
                value: state.value - 1
            }
        }
    }

    #[tokio::test]
    async fn counter_default_state() {
        let store = Store::new(counter_reducer);
        assert_eq!(Counter::default(), store.state_cloned().await);
    }

    #[tokio::test]
    async fn counter_supplied_state() {
        let store = Store::new_with_state(counter_reducer, Counter::new(5));
        assert_eq!(Counter::new(5), store.state_cloned().await);
    }

    #[tokio::test]
    async fn counter_actions_cloned_state() {
        let store = Store::new(counter_reducer);
        assert_eq!(Counter::new(42), store.state_cloned().await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(Counter::new(43), store.state_cloned().await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(Counter::new(44), store.state_cloned().await);

        store.dispatch(CounterAction::Decrement).await;
        assert_eq!(Counter::new(43), store.state_cloned().await);
    }

    #[tokio::test]
    async fn counter_actions_selector_struct() {
        let store = Store::new(counter_reducer);
        assert_eq!(42, store.select(ValueSelector).await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(43, store.select(ValueSelector).await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(44, store.select(ValueSelector).await);

        store.dispatch(CounterAction::Decrement).await;
        assert_eq!(43, store.select(ValueSelector).await);
    }

    #[tokio::test]
    async fn counter_actions_selector_lambda() {
        let store = Store::new(counter_reducer);
        assert_eq!(42, store.select(|state: &Counter| state.value).await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(43, store.select(|state: &Counter| state.value).await);

        store.dispatch(CounterAction::Increment).await;
        assert_eq!(44, store.select(|state: &Counter| state.value).await);

        store.dispatch(CounterAction::Decrement).await;
        assert_eq!(43, store.select(|state: &Counter| state.value).await);
    }
}
