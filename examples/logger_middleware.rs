// Configure logger middleware and observe several actions with unchanged state.
// Run with: cargo run --example logger_middleware --features logger

use log::{Level, LevelFilter};
use redux_rs::{DispatchError, Store, middlewares::logger::LoggerMiddleware};

#[derive(Default)]
struct EmptyStore;

#[derive(Debug)]
struct LoggableAction(&'static str);

fn nop_reducer(state: EmptyStore, _action: &LoggableAction) -> EmptyStore {
    state
}

fn main() -> Result<(), DispatchError> {
    // Install the application's logger and enable debug output, so this example
    // works without setting RUST_LOG=debug. The middleware does not install one.
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    // Select the middleware's log level independently of the logger's filter.
    let logger_middleware = LoggerMiddleware::new(Level::Debug);
    let store = Store::builder(nop_reducer).wrap(logger_middleware).build();

    // Each action appears in the logs even though the reducer leaves state
    // unchanged. Forwarding also preserves Redux-style dispatch return values.
    for message in ["First action", "Second action", "Third action"] {
        let dispatched = store.dispatch(LoggableAction(message))?;
        assert_eq!(dispatched.0, message);
    }
    Ok(())
}
