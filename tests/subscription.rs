use redux_rs::{Store};

type State = i8;

#[derive(Clone, Copy)]
enum Action {
    Increment,
    Decrement
}

fn reducer(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => state + 1,
        Action::Decrement => state - 1
    }
}

#[test]
fn subscription_increment() {
    let mut store = Store::new(reducer, 0);
    let listener = |state: &State| {
        assert_eq!(*state, 1);
    };
    store.subscribe(listener);
    store.dispatch(Action::Increment);
}

#[test]
fn subscription_decrement() {
    let mut store = Store::new(reducer, 0);
    let listener = |state: &State| {
        assert_eq!(*state, -1);
    };
    store.subscribe(listener);
    store.dispatch(Action::Decrement);
}

#[test]
fn subscription_unsubscribe() {
    let mut store = Store::new(reducer, 0);
    let zeroth_listener = |state: &State| {
        assert!(*state <= 2);
    };
    let listener = |state: &State| {
        assert_eq!(*state, 1);
    };
    let second_listener = |state: &State| {
        assert_eq!(*state, 2);
    };
    // Add and remove subscribers in a non-consecutive order.
    let zeroth = store.subscribe(zeroth_listener);
    let index = store.subscribe(listener);
    store.dispatch(Action::Increment);
    let second_index = store.subscribe(second_listener);
    store.unsubscribe(index);
    store.dispatch(Action::Increment);
    store.unsubscribe(second_index);
    store.unsubscribe(zeroth);
    store.dispatch(Action::Increment);
}