use redux_rs::{combine_reducers, Store};

type State = i8;

enum Action {
    Increment,
    Decrement
}

fn reducer_counter(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => state + 1,
        Action::Decrement => state - 1
    }
}

fn reducer_take_two(state: &State, _: &Action) -> State {
    state * 2
}

#[test]
fn combine_increment() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_counter, reducer_take_two),
        0
    );
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 6);
}

#[test]
fn combine_increment_reverse() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_take_two, reducer_counter),
        0
    );
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    assert_eq!(*store.state(), 3);
}

#[test]
fn combine_decrement() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_counter, reducer_take_two),
        0
    );
    store.dispatch(Action::Decrement);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), -6);
}

#[test]
fn combine_decrement_reverse() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_take_two, reducer_counter),
        0
    );
    store.dispatch(Action::Decrement);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), -3);
}

#[test]
fn combine_mixed() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_counter, reducer_take_two),
        0
    );
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), 10);
}

#[test]
fn combine_mixed_reverse() {
    let mut store = Store::new(
        combine_reducers!(State, &Action, reducer_take_two, reducer_counter),
        0
    );
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Decrement);
    assert_eq!(*store.state(), 5);
}
