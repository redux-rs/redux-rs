use log::{Level, LevelFilter};
use redux_rs::middlewares::logger::LoggerMiddleware;
use redux_rs::{Store, StoreApi};

// Example showing how to use the logger middleware
#[derive(Default)]
struct EmptyStore;

#[derive(Debug)]
struct LogableAction(&'static str);

fn nop_reducer(store: EmptyStore, _action: LogableAction) -> EmptyStore {
    store
}

#[tokio::main]
async fn main() {
    // Enable env logger and set the default log level to debug
    // This way we don't need to run the example with RUST_LOG=debug
    env_logger::builder().filter(None, LevelFilter::Debug).init();

    // Setup the logger middleware with default "Debug" log level
    let logger_middleware = LoggerMiddleware::new(Level::Debug);

    // Create a new store and wrap it with the logger middleware
    let store = Store::new(nop_reducer).wrap(logger_middleware).await;

    // Dispatch some actions
    // Notice how every action is shown in the logs
    store.dispatch(LogableAction("First action")).await;
    store.dispatch(LogableAction("Second action")).await;
    store.dispatch(LogableAction("Third action")).await;
}
