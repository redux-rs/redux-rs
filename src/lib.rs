use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, Notify};
use tokio::task::JoinHandle;

pub struct Store<State, Action, RootReducer> {
    sender: UnboundedSender<Work<State, Action>>,
    _worker_handle: JoinHandle<()>,

    _types: PhantomData<RootReducer>
}

pub trait Reducer<State, Action> {
    fn reduce(&self, state: State, action: Action) -> State;
}

impl<F, State, Action> Reducer<State, Action> for F
where
    F: Fn(State, Action) -> State
{
    fn reduce(&self, state: State, action: Action) -> State {
        self(state, action)
    }
}

pub trait Selector<State> {
    type Result;

    fn select(&self, state: &State) -> Self::Result;
}

impl<F, State, Result> Selector<State> for F
where
    F: Fn(&State) -> Result
{
    type Result = Result;

    fn select(&self, state: &State) -> Self::Result {
        self(state)
    }
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

struct StateWorker<State, Action, RootReducer> {
    receiver: UnboundedReceiver<Work<State, Action>>,
    root_reducer: RootReducer,
    state: Option<State>
}

enum Work<State, Action> {
    Reduce(ReduceWork<Action>),
    Select(Box<dyn SelectWork<State> + Send + 'static>)
}

impl<State, Action> Work<State, Action> {
    fn reduce(action: Action) -> (Self, Arc<Notify>) {
        let (work, notifier) = ReduceWork::new(action);
        (Work::Reduce(work), notifier)
    }

    fn select<S, Result>(selector: S) -> (Self, oneshot::Receiver<Result>)
    where
        S: Selector<State, Result = Result> + Send + 'static,
        State: Send + 'static,
        Result: Send + 'static
    {
        let (work, result_receiver) = SelectWorkImpl::new(selector);
        (Work::Select(Box::new(work)), result_receiver)
    }
}

struct ReduceWork<Action> {
    action: Action,
    notify: Arc<Notify>
}

impl<Action> ReduceWork<Action> {
    pub fn new(action: Action) -> (Self, Arc<Notify>) {
        let notify = Arc::new(Notify::new());
        (
            ReduceWork {
                action,
                notify: notify.clone()
            },
            notify
        )
    }
}

trait SelectWork<State> {
    fn select(self: Box<Self>, state: &State);
}

struct SelectWorkImpl<Select, State, Result>
where
    Select: Selector<State, Result = Result>
{
    selector: Select,
    result_sender: oneshot::Sender<Result>,

    _type: PhantomData<State>
}

impl<Select, State, Result> SelectWorkImpl<Select, State, Result>
where
    Select: Selector<State, Result = Result>
{
    pub fn new(selector: Select) -> (Self, oneshot::Receiver<Result>) {
        let (result_sender, result_receiver) = oneshot::channel();

        (
            SelectWorkImpl {
                selector,
                result_sender,

                _type: Default::default()
            },
            result_receiver
        )
    }
}

impl<Select, State, Result> SelectWork<State> for SelectWorkImpl<Select, State, Result>
where
    Select: Selector<State, Result = Result>
{
    fn select(self: Box<Self>, state: &State) {
        let selection = self.selector.select(state);
        let _ = self.result_sender.send(selection);
    }
}

impl<State, Action, RootReducer> StateWorker<State, Action, RootReducer>
where
    RootReducer: Reducer<State, Action>
{
    pub fn new(
        root_reducer: RootReducer,
        state: State
    ) -> (Self, UnboundedSender<Work<State, Action>>) {
        let (sender, receiver) = unbounded_channel();

        (
            StateWorker {
                receiver,
                root_reducer,
                state: Some(state)
            },
            sender
        )
    }

    pub async fn run(&mut self) {
        while let Some(work) = self.receiver.recv().await {
            match work {
                Work::Reduce(reduce_work) => self.reduce(reduce_work),
                Work::Select(select_work) => select_work.select(self.state.as_ref().unwrap())
            }
        }
    }

    fn reduce(&mut self, reduce_work: ReduceWork<Action>) {
        let ReduceWork { action, notify } = reduce_work;

        let old_state = self.state.take().unwrap();
        let new_state = self.root_reducer.reduce(old_state, action);

        self.state = Some(new_state);

        notify.notify_one()
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
