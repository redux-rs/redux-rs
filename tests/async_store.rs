#![cfg(feature = "tokio")]

use redux_rs::{DispatchError, Store};
use std::{
    future::Future,
    rc::Rc,
    task::{Context, Waker},
};
use tokio::task::LocalSet;

#[tokio::test]
async fn mailbox_supports_remote_tasks_and_explicit_shutdown() {
    LocalSet::new()
        .run_until(async {
            let store = Store::new(|state: i32, action: &i32| state + action).into_async(4);
            let other = store.clone();
            assert_eq!(
                tokio::spawn(async move { other.dispatch(2_i32).await })
                    .await
                    .unwrap(),
                Ok(2)
            );
            assert_eq!(store.select(|state: &i32| *state).await, Ok(2));
            let closed = store.clone();
            store.shutdown().await.unwrap();
            assert_eq!(
                closed.dispatch(1_i32).await,
                Err(DispatchError::WorkerStopped)
            );
        })
        .await;
}

#[tokio::test]
async fn dropping_all_handles_drains_cancelled_requests_and_releases_state() {
    LocalSet::new()
        .run_until(async {
            let value = Rc::new(());
            let weak = Rc::downgrade(&value);
            let count = Rc::new(std::cell::Cell::new(0));
            let capture = count.clone();
            let store = Store::new_with_state(
                move |state: Rc<()>, _: &()| {
                    capture.set(capture.get() + 1);
                    state
                },
                value,
            )
            .into_async(1);
            let mut request = Box::pin(store.dispatch(()));
            assert!(
                request
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            drop(request); // Work was enqueued, so it must still execute.
            drop(store);
            tokio::task::yield_now().await;
            assert_eq!(count.get(), 1);
            assert!(weak.upgrade().is_none());
        })
        .await;
}

#[tokio::test]
async fn worker_panic_returns_errors_to_waiting_and_future_callers() {
    LocalSet::new()
        .run_until(async {
            let store =
                Store::new(|_: i32, _: &()| -> i32 { panic!("reducer failed") }).into_async(1);
            assert_eq!(store.dispatch(()).await, Err(DispatchError::WorkerStopped));
            assert_eq!(store.dispatch(()).await, Err(DispatchError::WorkerStopped));
        })
        .await;
}
