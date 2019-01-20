[![Build Status][build-img]][build-url]
[![Crates.io][crates-io-img]][crates-io-url]
[![Documentation][docs-img]][docs-url]
[![Code Coverage][coverage-img]][coverage-url]

# redux-rs

> A Rust implementation of Redux.

## Redux

[Redux][redux-wikipedia-url], [originally implemented in JavaScript][redux-js-url], is an functional approach to manage a state.
The core concept is that you have a _state_ and a _reducer_, a function to create a new state from the old one and an _action_, a description of what to change.
Because the state itself is immutable, this results in a very clean way of managing application state, where every possible action is defined beforehand and dispatched later on.

## Usage

You might want to read the [documentation][docs-url], which also provides examples.

Also consider checking out the [examples](examples).

To run an example:

```
cargo run --example <name of the example>
```

To jump right into it, here is the simple counter example from [examples/counter.rs](examples/counter.rs):

```rust
use redux_rs::{Store, Subscription};

#[derive(Default)]
// This is a state. It describes an immutable object.
// It is changed via a 'reducer', a function which receives an action and returns a new state modified based on the action.
struct State {
    counter: i8
}

// The actions describe what the reducer has to do.
// Rust enums can carry a payload, which one can use to pass some value to the reducer.
enum Action {
    Increment,
    Decrement
}

// Here comes the reducer. It gets the current state plus an action to perform and returns a new state.
fn counter_reducer(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => State {
            counter: state.counter + 1
        },
        Action::Decrement => State {
            counter: state.counter - 1
        }
    }
}

fn main() {
    // A store is a way to handle a state. It gets created once and after that it can be read and changed via dispatching actions.
    let mut store = Store::new(counter_reducer, State::default());

    // A listener getting triggered whenever the state changes.
    let listener: Subscription<State> = |state: &State| {
        println!("Counter changed! New value: {}", state.counter);
    };

    // Listener gets subscribed to the store.
    store.subscribe(listener);

    // Now, let's dispatch some actions!
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Decrement);
    store.dispatch(Action::Decrement);

    // Retrieve the value at any time.
    println!("Final value: {}", store.state().counter);
}
```

### `no_std` support

redux-rs supports the `no_std` feature via disabling the default features.

_**Note:**_ This requires a nightly compiler and the availability of the `alloc` crate for the target.

In your `Cargo.toml`:

```toml
[dependencies]
redux-rs = { version = "...", default-features = false }
```

## Benchmarks

Running benchmarks requires a nightly compiler.

```
cargo +nightly bench
```

```
test counter_decrement                         ... bench:           2 ns/iter (+/- 0)
test counter_increment_with_reverse_middleware ... bench:           6 ns/iter (+/- 0)
test counter_increment_with_subscription       ... bench:           3 ns/iter (+/- 0)
```

[build-img]: https://travis-ci.com/redux-rs/redux-rs.svg?branch=master
[build-url]: https://travis-ci.com/redux-rs/redux-rs
[crates-io-img]: https://img.shields.io/crates/v/redux-rs.svg
[crates-io-url]: https://crates.io/crates/redux-rs
[docs-img]: https://docs.rs/redux-rs/badge.svg
[docs-url]: https://docs.rs/redux-rs
[coverage-img]: https://codecov.io/gh/redux-rs/redux-rs/branch/master/graph/badge.svg
[coverage-url]: https://codecov.io/gh/redux-rs/redux-rs
[redux-wikipedia-url]: https://en.wikipedia.org/wiki/Redux_(JavaScript_library)
[redux-js-url]: https://redux.js.org
