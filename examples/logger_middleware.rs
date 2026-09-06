use log::{Level, LevelFilter};
use redux_rs::{DispatchError, Store, middlewares::logger::LoggerMiddleware};

#[derive(Debug)]
enum Action {
    Increment,
}

fn main() -> Result<(), DispatchError> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();
    let store = Store::builder(|state: i32, _: &Action| state + 1)
        .wrap(LoggerMiddleware::new(Level::Debug))
        .build();
    store.dispatch(Action::Increment)?;
    assert_eq!(store.get_state(), 1);
    Ok(())
}
