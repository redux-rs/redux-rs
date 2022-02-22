use crate::store::worker::Work;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct Mailbox<State, Action> {
    rx: UnboundedReceiver<Work<State, Action>>,
    tx: UnboundedSender<Work<State, Action>>
}

impl<State, Action> Mailbox<State, Action> {
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        Mailbox { rx, tx }
    }

    pub fn address(&self) -> Address<State, Action> {
        Address::new(self.tx.clone())
    }

    pub async fn recv(&mut self) -> Option<Work<State, Action>> {
        self.rx.recv().await
    }
}

#[derive(Clone)]
pub struct Address<State, Action> {
    tx: UnboundedSender<Work<State, Action>>
}

impl<State, Action> Address<State, Action> {
    fn new(tx: UnboundedSender<Work<State, Action>>) -> Self {
        Address { tx }
    }

    pub fn send(&self, work: Work<State, Action>) {
        _ = self.tx.send(work);
    }
}
