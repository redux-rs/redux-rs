use async_trait::async_trait;
use tokio::sync::oneshot::Sender;

// Work trait, defines the result of the work
pub trait Work
where
    Self: Send,
{
    type Result: Send;
}

// Handle the work, return W::Result as a result
#[async_trait]
pub trait HandleWork<W>
where
    W: Work,
{
    async fn handle_work(&mut self, work: W) -> W::Result;
}

pub struct StateWorkerMessage<W>
where
    W: Work,
{
    work: W,
    callback: Sender<W::Result>,
}

impl<W> StateWorkerMessage<W>
where
    W: Work,
{
    pub fn new(work: W, callback: Sender<W::Result>) -> Self {
        StateWorkerMessage { work, callback }
    }
}

#[async_trait]
pub trait UnitOfWork<W> {
    async fn execute(self: Box<Self>, work_handler: &mut W);
}

#[async_trait]
impl<WorkHandler, W> UnitOfWork<WorkHandler> for StateWorkerMessage<W>
where
    WorkHandler: HandleWork<W> + Send,
    W: Work + Send,
    Self: Send,
{
    async fn execute(self: Box<Self>, work_handler: &mut WorkHandler) {
        let result = work_handler.handle_work(self.work).await;
        let _ = self.callback.send(result);
    }
}
