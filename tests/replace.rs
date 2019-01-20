use redux_rs::Store;

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

fn double_reducer(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => state + 2,
        Action::Decrement => state - 2
    }
}

#[test]
fn replace_increment() {
    let mut store = Store::new(reducer, 0);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 1);
    store.replace_reducer(double_reducer);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 3);
}

#[test]
fn replace_decrement() {
    let mut store = Store::new(reducer, 0);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), -1);
    store.replace_reducer(double_reducer);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), -3);
}
