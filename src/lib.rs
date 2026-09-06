//! Typed Redux-style state management, with synchronous dispatch and composable middleware.
//!
//! ```
//! use redux_rs::Store;
//!
//! #[derive(Debug, PartialEq)]
//! enum Action { Increment }
//! let store = Store::new(|state: i32, action: &Action| match action {
//!     Action::Increment => state + 1,
//! });
//! let subscription = store.subscribe(|state: &i32| println!("Count: {state}"));
//! assert_eq!(store.dispatch(Action::Increment)?, Action::Increment);
//! assert_eq!(store.get_state(), 1);
//! subscription.unsubscribe();
//! # Ok::<(), redux_rs::DispatchError>(())
//! ```
//!
//! Use `.middleware(...)` for closures that preserve the current action type,
//! and `.wrap(middleware(...))` to introduce a new type with `From<PreviousInput>`.
//! Earlier action types remain dispatchable through the complete stack.
//! Middleware receives separate [`MiddlewareApi`] and [`Next`] dispatchers.
//!
//! Stores and callbacks are local to a thread. Async thunks need only an executor;
//! the core has no Tokio dependency. Reducers must stay pure and synchronous.
//!
//! Unsupported action types are rejected at compile time:
//!
//! ```compile_fail
//! use redux_rs::Store;
//! let store = Store::new(|state: i32, action: &i32| state + action);
//! store.dispatch("not an integer action");
//! ```

#[cfg(feature = "tokio")]
mod async_store;
mod error;
mod middleware;
pub mod middlewares;
mod reducer;
mod selector;
mod store;
mod subscriber;

#[cfg(feature = "tokio")]
pub use async_store::AsyncStore;
pub use error::{DispatchError, DispatchResult};
pub use middleware::{
    DispatchInput, Extended, InputSet, Middleware, MiddlewareApi, MiddlewareFn, Next, Single,
    middleware,
};
pub use reducer::Reducer;
pub use selector::Selector;
pub use store::{Store, StoreBuilder};
pub use subscriber::{Subscriber, Subscription};
