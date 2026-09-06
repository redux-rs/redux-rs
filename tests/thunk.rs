#![cfg(feature = "thunk")]

use redux_rs::{
    DispatchError, Store, middleware,
    middlewares::thunk::{ThunkMiddleware, thunk},
};
use std::{
    cell::RefCell,
    future::Future,
    rc::Rc,
    task::{Context, Waker},
};

#[derive(Debug, PartialEq)]
struct Action(i32);
struct Logged(Action);
impl From<Action> for Logged {
    fn from(action: Action) -> Self {
        Self(action)
    }
}

#[tokio::test]
async fn thunks_preserve_results_and_reenter_outer_middleware() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let capture = events.clone();
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .wrap(ThunkMiddleware)
        .wrap(middleware(move |_, next, action: Logged| {
            capture.borrow_mut().push(action.0.0);
            next.dispatch(action.0)
        }))
        .build();
    let owned = String::from("consumed");
    let borrowed = &owned;
    let result = store
        .dispatch(thunk(move |api| async move {
            assert_eq!(borrowed, "consumed"); // Future need not be 'static.
            api.dispatch(Action(1))?;
            let length = api
                .dispatch(thunk(move |api| async move {
                    api.dispatch(Logged(Action(2)))?;
                    Ok::<_, DispatchError>(borrowed.len())
                }))
                .await?;
            Ok::<_, DispatchError>((length, api.get_state()))
        }))
        .await
        .unwrap();
    assert_eq!(result, (8, 3));
    assert_eq!(*events.borrow(), [1, 2]);
}

#[tokio::test]
async fn consuming_thunks_await_completion_and_keep_store_alive() {
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .wrap(ThunkMiddleware)
        .build();
    let api = store.api();
    let (reply, receive) = tokio::sync::oneshot::channel::<Box<i32>>();
    let owned = String::from("moved into the future");
    let mut future = Box::pin(store.dispatch(thunk(move |api| async move {
        let consumed = owned;
        let value = receive.await.unwrap();
        api.dispatch(Action(*value))?;
        Ok::<_, DispatchError>((consumed, api.get_state()))
    })));
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(store.get_state(), 0);
    drop(store);
    assert_eq!(api.get_state(), 0);
    reply.send(Box::new(7)).unwrap();
    assert_eq!(future.await.unwrap(), ("moved into the future".into(), 7));
    assert_eq!(api.dispatch(Action(1)), Err(DispatchError::StoreDropped));
}

#[tokio::test]
async fn thunk_errors_and_cancellation_are_observable() {
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .wrap(ThunkMiddleware)
        .build();
    let result = store
        .dispatch(thunk(|_| async {
            Err::<(), _>(DispatchError::Rejected("request failed".into()))
        }))
        .await;
    assert_eq!(
        result,
        Err(DispatchError::Rejected("request failed".into()))
    );
    let api = store.api();
    let future = store.dispatch(thunk(|api| async move {
        std::future::pending::<()>().await;
        api.dispatch(Action(1))?;
        Ok::<_, DispatchError>(())
    }));
    drop(store);
    drop(future);
    assert_eq!(api.dispatch(Action(1)), Err(DispatchError::StoreDropped));
    assert_eq!(
        api.dispatch(thunk(|_| async { Ok::<_, DispatchError>(()) }))
            .await,
        Err(DispatchError::StoreDropped)
    );
}

#[tokio::test]
async fn application_errors_keep_their_type() {
    #[derive(Debug, PartialEq)]
    enum AppError {
        Store(DispatchError),
        Offline,
    }
    impl From<DispatchError> for AppError {
        fn from(error: DispatchError) -> Self {
            Self::Store(error)
        }
    }
    let store = Store::builder(|state: i32, _: &Action| state)
        .wrap(ThunkMiddleware)
        .build();
    let result = store
        .dispatch(thunk(|api| async move {
            api.dispatch(Action(1))?;
            Err::<i32, _>(AppError::Offline)
        }))
        .await;
    assert_eq!(result, Err(AppError::Offline));
}
