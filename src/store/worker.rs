use crate::{Reducer, Selector};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, Notify};

pub struct StateWorker<State, Action, RootReducer> {
    receiver: UnboundedReceiver<Work<State, Action>>,
    root_reducer: RootReducer,
    state: Option<State>
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

pub enum Work<State, Action> {
    Reduce(ReduceWork<Action>),
    Select(Box<dyn SelectWork<State> + Send + 'static>)
}

impl<State, Action> Work<State, Action> {
    pub fn reduce(action: Action) -> (Self, Arc<Notify>) {
        let (work, notifier) = ReduceWork::new(action);
        (Work::Reduce(work), notifier)
    }

    pub fn select<S, Result>(selector: S) -> (Self, oneshot::Receiver<Result>)
    where
        S: Selector<State, Result = Result> + Send + 'static,
        State: Send + 'static,
        Result: Send + 'static
    {
        let (work, result_receiver) = SelectWorkImpl::new(selector);
        (Work::Select(Box::new(work)), result_receiver)
    }
}

pub struct ReduceWork<Action> {
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

pub trait SelectWork<State> {
    fn select(self: Box<Self>, state: &State);
}

pub struct SelectWorkImpl<Select, State, Result>
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
