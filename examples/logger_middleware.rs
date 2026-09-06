use log::{Level, LevelFilter};
use redux_rs::middlewares::logger::LoggerMiddleware;
use redux_rs::{DispatchError, Store};

// Example showing how to use the logger middleware
#[derive(Default)]
struct EmptyStore;

#[derive(Debug)]
struct LogableAction(&'static str);

fn nop_reducer(store: EmptyStore, _action: &LogableAction) -> EmptyStore {
    store
}

fn main() -> Result<(), DispatchError> {
    // Enable env logger and set the default log level to debug
    // This way we don't need to run the example with RUST_LOG=debug
    env_logger::builder()
        .filter(None, LevelFilter::Debug)
        .init();

    // Setup the logger middleware with default "Debug" log level
    let logger_middleware = LoggerMiddleware::new(Level::Debug);

    // Create a new store and wrap it with the logger middleware
    let store = Store::builder(nop_reducer).wrap(logger_middleware).build();

    // Dispatch some actions
    // Notice how every action is shown in the logs
    let action = store.dispatch(LogableAction("First action"))?;
    store.dispatch(LogableAction("Second action"))?;
    store.dispatch(LogableAction("Third action"))?;
    assert_eq!(action.0, "First action");
    Ok(())
}
