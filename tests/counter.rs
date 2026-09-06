use redux_rs::{DispatchError, Selector, Store, middleware};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[derive(Debug, PartialEq)]
enum Action {
    Increment,
    Set(i32),
}

fn reducer(state: i32, action: &Action) -> i32 {
    match action {
        Action::Increment => state + 1,
        Action::Set(value) => *value,
    }
}

struct Count;
impl Selector<i32> for Count {
    type Result = i32;
    fn select(self, state: &i32) -> i32 {
        *state
    }
}

#[test]
fn synchronous_dispatch_and_consuming_selectors() {
    let store = Store::new_with_state(reducer, 42);
    assert_eq!(store.dispatch(Action::Increment), Ok(Action::Increment));
    assert_eq!(store.select(Count), 43);
    let captured = String::from("count");
    assert_eq!(
        store.select(|state: &i32| (captured, *state)),
        ("count".into(), 43)
    );
    let local = 2;
    assert_eq!(store.select(|state: &i32| state + local), 45);
}

struct Logged {
    action: Action,
    level: u8,
}
impl From<Action> for Logged {
    fn from(action: Action) -> Self {
        Self { action, level: 1 }
    }
}
struct Traced(Logged);
impl From<Logged> for Traced {
    fn from(action: Logged) -> Self {
        Self(action)
    }
}

#[test]
fn old_and_new_inputs_promote_through_multiple_wrappers() {
    let logs = Rc::new(RefCell::new(Vec::new()));
    let captured = logs.clone();
    let store = Store::builder(reducer)
        .wrap(middleware(move |_, next, input: Logged| {
            captured.borrow_mut().push(input.level);
            next.dispatch(input.action)
        }))
        .wrap(middleware(|_, next, input: Traced| next.dispatch(input.0)))
        .build();
    store.dispatch(Action::Increment).unwrap();
    store
        .dispatch(Logged {
            action: Action::Increment,
            level: 2,
        })
        .unwrap();
    store
        .dispatch(Traced(Logged {
            action: Action::Increment,
            level: 3,
        }))
        .unwrap();
    assert_eq!(store.get_state(), 3);
    assert_eq!(*logs.borrow(), [1, 2, 3]);
}

#[test]
fn middleware_order_and_full_chain_redispatch() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let inner = log.clone();
    let outer = log.clone();
    let store = Store::builder(reducer)
        .middleware(move |api, next, action| {
            inner.borrow_mut().push("inner before");
            if action == Action::Set(99) {
                api.dispatch(Action::Increment)?;
            }
            let result = next.dispatch(action);
            inner.borrow_mut().push("inner after");
            result
        })
        .middleware(move |_, next, action| {
            outer.borrow_mut().push("outer before");
            let result = next.dispatch(action);
            outer.borrow_mut().push("outer after");
            result
        })
        .build();
    store.dispatch(Action::Set(99)).unwrap();
    assert_eq!(
        *log.borrow(),
        [
            "outer before",
            "inner before",
            "outer before",
            "inner before",
            "inner after",
            "outer after",
            "inner after",
            "outer after"
        ]
    );
    assert_eq!(store.get_state(), 99);
}

#[test]
fn middleware_can_transform_drop_repeat_and_return_typed_values() {
    let notifications = Rc::new(Cell::new(0));
    let count = notifications.clone();
    let store = Store::builder(reducer)
        .middleware(|api, next, action| {
            match action {
                Action::Set(0) => return Ok(0_usize), // Swallow without notifying.
                Action::Set(-1) => return Err(DispatchError::Rejected("negative".into())),
                Action::Set(2) => {
                    next.dispatch(Action::Increment)?;
                    next.dispatch(Action::Increment)?;
                }
                action => {
                    next.dispatch(action)?;
                }
            }
            Ok(api.get_state() as usize)
        })
        .build();
    let _subscription = store.listen(move || count.set(count.get() + 1));
    assert_eq!(store.dispatch(Action::Set(0)), Ok(0));
    assert_eq!(
        store.dispatch(Action::Set(-1)),
        Err(DispatchError::Rejected("negative".into()))
    );
    assert_eq!(notifications.get(), 0);
    assert_eq!(store.dispatch(Action::Set(2)), Ok(2));
    assert_eq!(notifications.get(), 2);
}

#[test]
fn listeners_can_redispatch_and_unsubscribe_with_snapshot_ordering() {
    let store = Store::new(reducer);
    let api = store.api();
    let events = Rc::new(RefCell::new(Vec::new()));
    let second = Rc::new(RefCell::new(None::<redux_rs::Subscription>));
    let remove_second = second.clone();
    let _first = store.listen(move || {
        if api.get_state() == 1 {
            remove_second.borrow().as_ref().unwrap().unsubscribe();
            api.dispatch(Action::Increment).unwrap();
        }
    });
    let captured = events.clone();
    *second.borrow_mut() =
        Some(store.subscribe(move |state: &i32| captured.borrow_mut().push(*state)));
    store.dispatch(Action::Increment).unwrap();
    // Removed listener still runs in the outer snapshot, sees the latest state.
    assert_eq!(*events.borrow(), [2]);
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(*events.borrow(), [2]);
}

#[test]
fn state_snapshot_survives_nested_dispatch_and_drop_unsubscribes() {
    let store = Store::new(reducer);
    let api = store.api();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let capture = seen.clone();
    let subscription = store.subscribe(move |state: &i32| {
        if *state == 1 {
            api.dispatch(Action::Increment).unwrap();
        }
        capture.borrow_mut().push(*state);
    });
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(*seen.borrow(), [2, 1]);
    drop(subscription);
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(*seen.borrow(), [2, 1]);
}

#[test]
fn non_clone_state_and_callbacks_release_with_last_store() {
    struct State(Rc<()>);
    let value = Rc::new(());
    let weak = Rc::downgrade(&value);
    let store = Store::new_with_state(|state: State, _: &()| state, State(value));
    assert_eq!(store.select(|state: &State| Rc::strong_count(&state.0)), 1);
    let subscription = store.listen(|| {});
    let api = store.api();
    let clone = store.clone();
    drop(store);
    assert!(weak.upgrade().is_some());
    drop(clone);
    assert!(weak.upgrade().is_none());
    assert_eq!(api.dispatch(()), Err(DispatchError::StoreDropped));
    subscription.unsubscribe();
    subscription.unsubscribe();
}

#[test]
fn selector_cannot_dispatch_and_reducer_panic_poison_is_explicit() {
    let store = Store::new(reducer);
    assert_eq!(
        store.select(|_: &i32| store.dispatch(Action::Increment)),
        Err(DispatchError::StateBorrowed)
    );
    assert_eq!(store.get_state(), 0);
    let broken = Store::new(|_: i32, _: &()| -> i32 { panic!("bad reducer") });
    assert!(catch_unwind(AssertUnwindSafe(|| broken.dispatch(()))).is_err());
    assert_eq!(broken.dispatch(()), Err(DispatchError::Poisoned));
    assert_eq!(
        broken.try_select(|state: &i32| *state),
        Err(DispatchError::Poisoned)
    );
}

#[test]
fn selector_dispatch_runs_middleware_but_cannot_reduce_borrowed_state() {
    let effects = Rc::new(Cell::new(0));
    let capture = effects.clone();
    let store = Store::builder(reducer)
        .middleware(move |_, next, action| {
            capture.set(capture.get() + 1);
            match action {
                Action::Set(0) => Ok(action),
                action => next.dispatch(action),
            }
        })
        .build();
    assert_eq!(
        store.select(|_: &i32| store.dispatch(Action::Set(0))),
        Ok(Action::Set(0))
    );
    assert_eq!(
        store.select(|_: &i32| store.dispatch(Action::Increment)),
        Err(DispatchError::StateBorrowed)
    );
    assert_eq!(effects.get(), 2);
    assert_eq!(store.get_state(), 0);
}

#[test]
fn reducer_redispatch_is_rejected_before_any_middleware_runs() {
    let callback = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
    let callback_in_reducer = callback.clone();
    let count = Rc::new(Cell::new(0));
    let capture = count.clone();
    let store = Store::builder(move |state: i32, _: &Action| {
        callback_in_reducer.borrow().as_ref().unwrap()();
        state + 1
    })
    .middleware(move |_, next, action| {
        capture.set(capture.get() + 1);
        next.dispatch(action)
    })
    .build();
    let api = store.api();
    *callback.borrow_mut() = Some(Box::new(move || {
        assert_eq!(
            api.dispatch(Action::Increment),
            Err(DispatchError::Reducing)
        );
        assert_eq!(
            api.try_select(|state: &i32| *state),
            Err(DispatchError::Reducing)
        );
    }));
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(store.get_state(), 1);
    assert_eq!(count.get(), 1);
}

#[test]
fn newly_subscribed_listeners_wait_for_the_next_dispatch() {
    let store = Store::new(reducer);
    let seen = Rc::new(Cell::new(0));
    let added = Rc::new(RefCell::new(None));
    let pending = added.clone();
    let cloned = store.clone();
    let capture = seen.clone();
    let first = store.listen(move || {
        if pending.borrow().is_none() {
            let capture = capture.clone();
            *pending.borrow_mut() = Some(cloned.listen(move || capture.set(capture.get() + 1)));
        }
    });
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(seen.get(), 0);
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(seen.get(), 1);
    // Break the deliberate strong capture in this application's callback.
    first.unsubscribe();
}

#[test]
fn middleware_can_retain_next_without_restarting_or_retaining_the_store() {
    let pending = Rc::new(RefCell::new(None));
    let capture = pending.clone();
    let store = Store::builder(reducer)
        .middleware(move |_, next, action| {
            *capture.borrow_mut() = Some((next.clone(), action));
            Ok(())
        })
        .build();
    store.dispatch(Action::Increment).unwrap();
    assert_eq!(store.get_state(), 0);
    let (next, action) = pending.borrow_mut().take().unwrap();
    assert_eq!(next.dispatch(action), Ok(Action::Increment));
    assert!(pending.borrow().is_none());
    assert_eq!(store.get_state(), 1);
    drop(store);
    assert_eq!(
        next.dispatch(Action::Increment),
        Err(DispatchError::StoreDropped)
    );
}

#[test]
fn inner_middleware_can_dispatch_an_action_type_added_later() {
    let store = Store::builder(reducer)
        .middleware(|api, next, action| {
            if action == Action::Set(99) {
                return api.dispatch(Traced(Logged {
                    action: Action::Increment,
                    level: 5,
                }));
            }
            next.dispatch(action)
        })
        .wrap(middleware(|_, next, input: Logged| {
            next.dispatch(input.action)
        }))
        .wrap(middleware(|_, next, input: Traced| next.dispatch(input.0)))
        .build();
    assert_eq!(store.dispatch(Action::Set(99)), Ok(Action::Increment));
    assert_eq!(store.get_state(), 1);
}
