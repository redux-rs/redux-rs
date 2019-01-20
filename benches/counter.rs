#![feature(test)]
extern crate test;

use redux_rs::Store;
use test::Bencher;

type State = i16;

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

#[bench]
fn counter_decrement(bencher: &mut Bencher) {
    let mut store = Store::new(reducer, 0);

    bencher.iter(|| {
        store.dispatch(Action::Decrement);
    });
}

#[bench]
fn counter_increment_with_subscription(bencher: &mut Bencher) {
    let mut store = Store::new(reducer, 0);

    store.subscribe(|state: &State| {
        let _ = state;
    });

    bencher.iter(|| {
        store.dispatch(Action::Increment);
    });
}

#[bench]
fn counter_increment_with_reverse_middleware(bencher: &mut Bencher) {
    let mut store = Store::new(reducer, 0);

    store.add_middleware(reverse_middleware);

    bencher.iter(|| {
        store.dispatch(Action::Decrement);
    });
}
