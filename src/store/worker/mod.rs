mod dispatch;
mod mailbox;
mod select;
mod work;

pub use dispatch::Dispatch;
pub use mailbox::{Address, Mailbox};
pub use select::Select;
pub use work::Work;

use crate::store::worker::work::HandleWork;
use crate::{Reducer, Selector};
use async_trait::async_trait;

pub struct StateWorker<State, Action, RootReducer>
where
    State: Send,
    RootReducer: Send
{
    mailbox: Mailbox<State, Action, RootReducer>,
    root_reducer: RootReducer,
    state: Option<State>
}

impl<State, Action, RootReducer> StateWorker<State, Action, RootReducer>
where
    RootReducer: Reducer<State, Action>,
    State: Send,
    RootReducer: Send
{
    pub fn new(root_reducer: RootReducer, state: State) -> Self {
        Self {
            mailbox: Mailbox::new(),
            root_reducer,
            state: Some(state)
        }
    }

    pub fn address(&self) -> Address<State, Action, RootReducer> {
        self.mailbox.address()
    }

    pub async fn run(&mut self) {
        while let Some(work) = self.mailbox.recv().await {
            work.execute(self).await;
        }
    }
}

#[async_trait]
impl<State, Action, RootReducer> HandleWork<Dispatch<Action>>
    for StateWorker<State, Action, RootReducer>
where
    RootReducer: Reducer<State, Action>,
    State: Send,
    RootReducer: Send,
    Action: Send
{
    async fn handle_work(&mut self, work: Dispatch<Action>) {
        let action = work.into_action();

        let old_state = self.state.take().unwrap();
        let new_state = self.root_reducer.reduce(old_state, action);

        self.state = Some(new_state);
    }
}

#[async_trait]
impl<State, Action, RootReducer, S, Result> HandleWork<Select<State, S>>
    for StateWorker<State, Action, RootReducer>
where
    RootReducer: Reducer<State, Action>,
    State: Send,
    RootReducer: Send,
    S: Selector<State, Result = Result> + Send + 'static,
    Result: Send
{
    async fn handle_work(&mut self, work: Select<State, S>) -> Result {
        let state = self.state.as_ref().unwrap();
        let selector = work.into_selector();
        selector.select(state)
    }
}
