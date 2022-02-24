use crate::store::worker::work::HandleWork;
use crate::store::worker::{
    work::{StateWorkerMessage, UnitOfWork, Work},
    StateWorker,
};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot::channel,
};

type Message<State, Action, RootReducer> = Box<dyn UnitOfWork<StateWorker<State, Action, RootReducer>> + Send>;

pub struct Mailbox<State, Action, RootReducer>
where
    State: Send,
    RootReducer: Send,
{
    rx: UnboundedReceiver<Message<State, Action, RootReducer>>,
    tx: UnboundedSender<Message<State, Action, RootReducer>>,
}

impl<State, Action, RootReducer> Mailbox<State, Action, RootReducer>
where
    State: Send,
    RootReducer: Send,
{
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        Mailbox { rx, tx }
    }

    pub fn address(&self) -> Address<State, Action, RootReducer> {
        Address::new(self.tx.clone())
    }

    pub async fn recv(&mut self) -> Option<Message<State, Action, RootReducer>> {
        self.rx.recv().await
    }
}

#[derive(Clone)]
pub struct Address<State, Action, RootReducer>
where
    State: Send,
    RootReducer: Send,
{
    tx: UnboundedSender<Message<State, Action, RootReducer>>,
}

impl<State, Action, RootReducer> Address<State, Action, RootReducer>
where
    State: Send,
    RootReducer: Send,
{
    fn new(tx: UnboundedSender<Message<State, Action, RootReducer>>) -> Self {
        Address { tx }
    }

    pub async fn send<W: Work + 'static>(&self, work: W) -> W::Result
    where
        StateWorker<State, Action, RootReducer>: HandleWork<W>,
    {
        let (tx, rx) = channel();
        let message = StateWorkerMessage::new(work, tx);
        let _ = self.tx.send(Box::new(message));
        rx.await.unwrap()
    }
}
