[![Build Status][build-img]][build-url]
[![Crates.io][crates-io-img]][crates-io-url]
[![Documentation][docs-img]][docs-url]

# redux-rs

> A Rust implementation of Redux.

## Redux

[Redux][redux-wikipedia-url], [originally implemented in JavaScript][redux-js-url], is an functional approach to state management.
The core concept is that you have a _state_ and a _reducer_, a function to create a new state from the old one and an _action_, a description of what to change.
Because the state itself is immutable, this results in a very clean way of managing application state, where every possible action is defined beforehand and dispatched later on.

## Usage

You might want to read the [documentation][docs-url], which also provides examples.

Also consider checking out the [examples](examples).

To run an example:

```
cargo run --example <name of the example>
```

[build-img]: https://github.com/jeroenvervaeke/redux-rs/actions/workflows/build.yml/badge.svg
[build-url]: https://github.com/jeroenvervaeke/redux-rs/actions/workflows/build.yml
[crates-io-img]: https://img.shields.io/crates/v/redux-rs.svg
[crates-io-url]: https://crates.io/crates/redux-rs
[docs-img]: https://docs.rs/redux-rs/badge.svg
[docs-url]: https://docs.rs/redux-rs
[redux-wikipedia-url]: https://en.wikipedia.org/wiki/Redux_(JavaScript_library)
[redux-js-url]: https://redux.js.org
