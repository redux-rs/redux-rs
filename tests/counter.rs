use redux_rs::Store;

type State = i8;

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
fn counter_increment() {
    let mut store = Store::new(reducer, 0);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 1);
}

#[test]
fn counter_decrement() {
    let mut store = Store::new(reducer, 0);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), -1);
}
