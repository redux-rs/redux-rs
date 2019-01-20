use redux_rs::{Store, Subscription};

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
    let listener: Subscription<State> = |state: &State| {
        assert_eq!(*state, 1);
    };
    store.subscribe(listener);
    store.dispatch(Action::Increment);
}

#[test]
fn subscription_decrement() {
    let mut store = Store::new(reducer, 0);
    let listener: Subscription<State> = |state: &State| {
        assert_eq!(*state, -1);
    };
    store.subscribe(listener);
    store.dispatch(Action::Decrement);
}
