//! # redux - A Rust implementation of Redux.
//!
//! Redux provides a clean way of managing states in an application.
//! It could be user data such as preferences or information about the state of the program.
//!
//! ## Concepts
//!
//! In Redux data is immutable. The only way to change it is to take it and create some new data by following a set of rules.
//!
//! ### State
//!
//! A state is the form of data Redux manages. Theoretically it can be anything, but for an easy explanation let's take the following example:
//! We have a simple counter application. It does nothing more than counting.
//! Our state would look the following:
//!
//! ```
//! #[derive(Default)]
//! struct State {
//!     counter: i8
//! }
//! ```
//!
//! ### Actions
//!
//! To change the state we need to dispatch actions. In Rust, they would usually be represented by an enum.
//! For the counter, we want to increment and decrement it.
//!
//! ```
//! enum Action {
//!     Increment,
//!     Decrement
//! }
//! ```
//!
//! ### Reducer
//!
//! To actually change the state (read: create a new one), we need what is called a reducer.
//! It is a simple function which takes in the current state plus the action to perform and returns a new state.
//!
//! ```
//! # struct State {
//! #     counter: i8
//! # }
//! #
//! # enum Action {
//! #     Increment,
//! #     Decrement
//! # }
//! #
//! fn reducer(state: &State, action: &Action) -> State {
//!     match action {
//!         Action::Increment => State {
//!             counter: state.counter + 1
//!         },
//!         Action::Decrement => State {
//!             counter: state.counter - 1
//!         }
//!     }
//! }
//! ```
//!
//! Note how the reducer uses the old data to create a new state.
//!
//! ### Store
//!
//! To put it all together, we use a store which keeps track of a state and provides an easy to use API for dispatching actions.
//! The store takes the reducer and an initial state.
//!
//! ```
//! # #[derive(Default)]
//! # struct State {
//! #     counter: i8
//! # }
//! #
//! # enum Action {
//! #     Increment,
//! #     Decrement
//! # }
//! #
//! # fn reducer(state: &State, action: &Action) -> State {
//! #     match action {
//! #         Action::Increment => State {
//! #             counter: state.counter + 1
//! #         },
//! #         Action::Decrement => State {
//! #             counter: state.counter - 1
//! #         }
//! #     }
//! # }
//! #
//! // The store needs to be mutable as it will change its inner state when dispatching actions.
//! let mut store = redux_rs::Store::new(reducer, State::default());
//!
//! // Let it do its highly complex math.
//! store.dispatch(Action::Increment);
//! store.dispatch(Action::Decrement);
//!
//! // Print the current count.
//! println!("{}", store.state().counter);
//! ```
//!
//! ### Subscriptions
//!
//! Sometimes one might want to listen to changes happening. This is where subscriptions come in.
//! They are callbacks with the current state that get called whenever an action gets dispatched.
//!
//! ```
//! # #[derive(Default)]
//! # struct State {
//! #     counter: i8
//! # }
//! #
//! # enum Action {
//! #     Increment,
//! #     Decrement
//! # }
//! #
//! # fn reducer(state: &State, action: &Action) -> State {
//! #     match action {
//! #         Action::Increment => State {
//! #             counter: state.counter + 1
//! #         },
//! #         Action::Decrement => State {
//! #             counter: state.counter - 1
//! #         }
//! #     }
//! # }
//! #
//! # let mut store = redux_rs::Store::new(reducer, State::default());
//! #
//! store.subscribe(|state: &State| {
//!      println!("Something changed! Current value: {}", state.counter);
//! });
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

mod middleware;
mod reducer;
mod store;
mod subscription;

pub use middleware::Middleware;
pub use reducer::Reducer;
#[cfg(not(feature = "devtools"))]
pub use store::Store;
pub use subscription::Subscription;
