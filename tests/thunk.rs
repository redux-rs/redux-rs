#![cfg(feature = "thunk")]

use redux_rs::{
    DispatchError, Store, middleware,
    middlewares::thunk::{ThunkMiddleware, ThunkOutcome, thunk},
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
    task::{Context, Poll, Waker},
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
        .thunk_middleware(|_, next, call| {
            let future = next.dispatch_thunk(call)?;
            Ok(Box::pin(async move {
                let outcome = future.await?;
                assert_eq!(outcome, ThunkOutcome::Failed);
                Ok(outcome)
            }))
        })
        .build();
    let result = store
        .dispatch(thunk(|api| async move {
            api.dispatch(Action(1))?;
            Err::<i32, _>(AppError::Offline)
        }))
        .await;
    assert_eq!(result, Err(AppError::Offline));
}

#[tokio::test]
async fn thunk_interceptors_follow_stack_order_and_wrap_borrowed_results() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let inner = events.clone();
    let outer = events.clone();
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .thunk_middleware(|_, _, _| panic!("ThunkMiddleware consumes calls before this layer"))
        .wrap(ThunkMiddleware)
        .thunk_middleware(move |_, next, call| {
            inner.borrow_mut().push("inner before");
            let future = next.dispatch_thunk(call)?;
            inner.borrow_mut().push("inner after");
            let events = inner.clone();
            Ok(Box::pin(async move {
                events.borrow_mut().push("inner polling");
                let outcome = future.await?;
                events.borrow_mut().push("inner complete");
                Ok(outcome)
            }))
        })
        .wrap(middleware(|_, next, input: Logged| next.dispatch(input.0)))
        .thunk_middleware(move |_, next, call| {
            outer.borrow_mut().push("outer before");
            let future = next.dispatch_thunk(call)?;
            outer.borrow_mut().push("outer after");
            let events = outer.clone();
            Ok(Box::pin(async move {
                events.borrow_mut().push("outer polling");
                let outcome = future.await?;
                events.borrow_mut().push("outer complete");
                Ok(outcome)
            }))
        })
        .build();
    let owned = String::from("borrowed result");
    let borrowed = owned.as_str();
    let capture = events.clone();
    let future = store.dispatch(thunk(move |api| {
        capture.borrow_mut().push("factory");
        async move {
            capture.borrow_mut().push("body");
            api.dispatch(Action(1))?;
            api.dispatch(Logged(Action(2)))?;
            Ok::<_, DispatchError>(borrowed)
        }
    }));
    assert_eq!(
        *events.borrow(),
        [
            "outer before",
            "inner before",
            "factory",
            "inner after",
            "outer after"
        ]
    );
    assert_eq!(future.await, Ok(borrowed));
    assert_eq!(store.get_state(), 3);
    assert_eq!(
        &events.borrow()[5..],
        [
            "outer polling",
            "inner polling",
            "body",
            "inner complete",
            "outer complete"
        ]
    );
}

#[tokio::test]
async fn thunk_rejection_prevents_factory_and_body_execution() {
    let factory_ran = Cell::new(false);
    let body_ran = Cell::new(false);
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .wrap(ThunkMiddleware)
        .thunk_middleware(|_, _, _| Err(DispatchError::Rejected("blocked".into())))
        .build();
    let result = store
        .dispatch(thunk(|_| {
            factory_ran.set(true);
            async {
                body_ran.set(true);
                Ok::<(), DispatchError>(())
            }
        }))
        .await;
    assert_eq!(result, Err(DispatchError::Rejected("blocked".into())));
    assert!(!factory_ran.get());
    assert!(!body_ran.get());
    // The interceptor does not accidentally reject ordinary actions.
    assert_eq!(store.dispatch(Action(1)), Ok(Action(1)));
}

#[tokio::test]
async fn thunk_interceptors_can_delay_factory_and_cancel_pending_work() {
    let (reply, receive) = tokio::sync::oneshot::channel::<()>();
    let gate = RefCell::new(Some(receive));
    let store = Store::builder(|state: i32, _: &Action| state)
        .wrap(ThunkMiddleware)
        .thunk_middleware(move |_, next, call| {
            let receive = gate.borrow_mut().take().unwrap();
            Ok(Box::pin(async move {
                receive.await.unwrap();
                next.dispatch_thunk(call)?.await
            }))
        })
        .build();
    let factory_ran = Cell::new(false);
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let ran = &factory_ran;
    let mut future = Box::pin(store.dispatch(thunk(move |_| {
        ran.set(true);
        async move { Ok::<_, DispatchError>(capture) }
    })));
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert!(!factory_ran.get());
    let api = store.api();
    drop(store);
    drop(future);
    assert!(!factory_ran.get());
    assert!(weak.upgrade().is_none());
    assert!(reply.send(()).is_err());
    assert_eq!(api.dispatch(Action(1)), Err(DispatchError::StoreDropped));
}

#[tokio::test]
async fn thunk_interceptors_can_reject_completion_but_cannot_fabricate_a_typed_value() {
    let store = Store::builder(|state: i32, action: &Action| state + action.0)
        .wrap(ThunkMiddleware)
        .thunk_middleware(|_, next, call| {
            let future = next.dispatch_thunk(call)?;
            Ok(Box::pin(async move {
                assert_eq!(future.await?, ThunkOutcome::Succeeded);
                Err(DispatchError::Rejected("result rejected".into()))
            }))
        })
        .build();
    let result = store
        .dispatch(thunk(|api| async move {
            api.dispatch(Action(1))?;
            Ok::<_, DispatchError>(42)
        }))
        .await;
    assert_eq!(
        result,
        Err(DispatchError::Rejected("result rejected".into()))
    );
    assert_eq!(store.get_state(), 1); // Rejection does not roll back applied actions.

    let store = Store::builder(|state: i32, _: &Action| state)
        .wrap(ThunkMiddleware)
        .thunk_middleware(|_, _, _| Ok(Box::pin(async { Ok(ThunkOutcome::Succeeded) })))
        .build();
    assert_eq!(
        store
            .dispatch(thunk(|_| async { Ok::<_, DispatchError>(42) }))
            .await,
        Err(DispatchError::Rejected(
            "thunk middleware completed without a result".into()
        ))
    );
}

#[test]
fn synchronous_thunk_rejection_releases_completed_result() {
    let state = Rc::new(());
    let weak = Rc::downgrade(&state);
    let store = Store::builder_with_state(|state: Rc<()>, _: &()| state, state)
        .wrap(ThunkMiddleware)
        .thunk_middleware(|_, next, call| {
            let mut task = next.dispatch_thunk(call)?;
            assert_eq!(
                task.as_mut().poll(&mut Context::from_waker(Waker::noop())),
                Poll::Ready(Ok(ThunkOutcome::Succeeded))
            );
            Err(DispatchError::Rejected("result rejected".into()))
        })
        .build();
    let api = store.api();
    let retained = store.clone();
    let mut future =
        Box::pin(store.dispatch(thunk(
            move |_| async move { Ok::<_, DispatchError>(retained) },
        )));
    drop(store);
    assert!(matches!(
        future.as_mut().poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(DispatchError::Rejected(reason))) if reason == "result rejected"
    ));
    // Keeping the completed future must not retain its discarded Store result.
    assert!(weak.upgrade().is_none());
    assert_eq!(api.dispatch(()), Err(DispatchError::StoreDropped));
    drop(future);
}
