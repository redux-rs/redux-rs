use std::{error::Error, fmt};

/// Failures in the store or an action rejected by middleware.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DispatchError {
    /// A weak middleware API outlived its store.
    StoreDropped,
    /// Reducers may not dispatch or read the store.
    Reducing,
    /// Reduction was attempted inside a selector that still borrows the state.
    StateBorrowed,
    /// A reducer panicked after taking ownership of the previous state.
    Poisoned,
    /// Middleware intentionally rejected an action.
    Rejected(String),
    /// The optional async worker stopped before replying.
    WorkerStopped,
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoreDropped => f.write_str("the store has been dropped"),
            Self::Reducing => f.write_str("reducers may not dispatch or read the store"),
            Self::StateBorrowed => f.write_str("cannot reduce while a selector borrows the state"),
            Self::Poisoned => f.write_str("the store lost its state because a reducer panicked"),
            Self::Rejected(reason) => write!(f, "action rejected: {reason}"),
            Self::WorkerStopped => f.write_str("the async store worker stopped before replying"),
        }
    }
}

impl Error for DispatchError {}

pub type DispatchResult<T> = Result<T, DispatchError>;
