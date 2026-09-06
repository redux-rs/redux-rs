//! Optional Tokio mailbox for accessing a local store from other tasks or threads.
//! Create it inside a Tokio `LocalSet`; only the handles and queued work cross
//! threads. Dropping the last handle drains queued work and releases the store.

use tokio::sync::{mpsc, oneshot};

use crate::{DispatchError, DispatchResult, InputSet, Promote, Selector, Single, Store};

type Work<State, Action, Output, Accepted> =
    Box<dyn FnOnce(&Store<State, Action, Output, Accepted>) + Send>;

enum Message<State, Action, Output, Accepted: InputSet<Input = Action>> {
    Work(Work<State, Action, Output, Accepted>),
    Shutdown(oneshot::Sender<()>),
}

/// A bounded mailbox for a store running on a Tokio `LocalSet`.
/// Actions are processed in mailbox order. Once enqueued, cancelling the caller's
/// future does not cancel the action. Async thunks run on the underlying local
/// store, not through this mailbox; this handle dispatches ordinary action inputs.
pub struct AsyncStore<
    State,
    Action,
    Output = Action,
    Accepted: InputSet<Input = Action> = Single<Action>,
> {
    sender: mpsc::Sender<Message<State, Action, Output, Accepted>>,
}

impl<State, Action, Output, Accepted: InputSet<Input = Action>> Clone
    for AsyncStore<State, Action, Output, Accepted>
{
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<State: 'static, Action: 'static, Output: 'static, Accepted: InputSet<Input = Action> + 'static>
    Store<State, Action, Output, Accepted>
{
    /// Move this handle into a local worker. Panics outside a Tokio `LocalSet`,
    /// or if `capacity` is zero. Other synchronous clones remain usable locally.
    pub fn into_async(self, capacity: usize) -> AsyncStore<State, Action, Output, Accepted> {
        let (sender, mut receiver) = mpsc::channel(capacity);
        tokio::task::spawn_local(async move {
            let mut shutdowns = Vec::new();
            while let Some(message) = receiver.recv().await {
                match message {
                    Message::Work(work) => work(&self),
                    Message::Shutdown(reply) => {
                        receiver.close();
                        shutdowns.push(reply);
                    }
                }
            }
            drop(self);
            for reply in shutdowns {
                let _ = reply.send(());
            }
        });
        AsyncStore { sender }
    }
}

impl<State: 'static, Action: 'static, Output: 'static, Accepted: InputSet<Input = Action> + 'static>
    AsyncStore<State, Action, Output, Accepted>
{
    async fn request<Result: Send + 'static>(
        &self,
        work: impl FnOnce(&Store<State, Action, Output, Accepted>) -> DispatchResult<Result>
        + Send
        + 'static,
    ) -> DispatchResult<Result> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(Message::Work(Box::new(move |store| {
                let _ = reply.send(work(store));
            })))
            .await
            .map_err(|_| DispatchError::WorkerStopped)?;
        result.await.map_err(|_| DispatchError::WorkerStopped)?
    }

    /// Enqueue an accepted action. Generic helpers can name the public
    /// `Accepted: Promote<Input, Path>` bound; `Path` is inferred by callers.
    pub async fn dispatch<Input: Send + 'static, Path>(
        &self,
        action: Input,
    ) -> DispatchResult<Output>
    where
        Accepted: Promote<Input, Path>,
        Output: Send,
    {
        self.request(move |store| store.dispatch_input(Accepted::promote(action)))
            .await
    }

    pub async fn select<S>(&self, selector: S) -> DispatchResult<S::Result>
    where
        S: Selector<State> + Send + 'static,
        S::Result: Send + 'static,
    {
        self.request(move |store| store.try_select(selector)).await
    }

    /// Close the mailbox to all handles, finish queued work, and release the
    /// worker's store handle. Returns an error if the worker previously panicked.
    pub async fn shutdown(self) -> DispatchResult<()> {
        let (reply, result) = oneshot::channel();
        self.sender
            .send(Message::Shutdown(reply))
            .await
            .map_err(|_| DispatchError::WorkerStopped)?;
        result.await.map_err(|_| DispatchError::WorkerStopped)
    }
}
