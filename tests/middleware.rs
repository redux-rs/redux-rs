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

fn reverse_middleware(_: &mut Store<State, Action>, action: Action) -> Option<Action> {
    match action {
        Action::Increment => Some(Action::Decrement),
        Action::Decrement => Some(Action::Increment)
    }
}

fn only_increment_middleware(_: &mut Store<State, Action>, action: Action) -> Option<Action> {
    match action {
        Action::Increment => Some(action),
        Action::Decrement => None
    }
}

#[test]
fn reverse_middleware_increment() {
    let mut store = Store::new(reducer, 0);
    store.add_middleware(reverse_middleware);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), -1);
}

#[test]
fn reverse_middleware_decrement() {
    let mut store = Store::new(reducer, 0);
    store.add_middleware(reverse_middleware);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), 1);
}

#[test]
fn only_increment_middleware_increment() {
    let mut store = Store::new(reducer, 0);
    store.add_middleware(only_increment_middleware);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 1);
}

#[test]
fn only_increment_middleware_decrement() {
    let mut store = Store::new(reducer, 0);
    store.add_middleware(only_increment_middleware);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), 0);
}
