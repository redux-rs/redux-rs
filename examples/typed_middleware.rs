use log::{Level, LevelFilter, log};
use redux_rs::{
    DispatchError, Store, middleware,
    middlewares::thunk::{ThunkMiddleware, thunk},
};

#[derive(Debug)]
enum Action {
    Increment,
    Set(i32),
}

struct ActionWithLogLevel {
    action: Action,
    level: Level,
}

impl From<Action> for ActionWithLogLevel {
    fn from(action: Action) -> Self {
        Self {
            action,
            level: Level::Info,
        }
    }
}

fn reducer(state: i32, action: &Action) -> i32 {
    match action {
        Action::Increment => state + 1,
        Action::Set(value) => *value,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), DispatchError> {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();
    let logger = middleware(|api, next, input: ActionWithLogLevel| {
        log!(input.level, "Dispatching {:?}", input.action);
        let result = next.dispatch(input.action);
        log!(input.level, "Count: {}", api.get_state());
        result
    });
    let store = Store::builder(reducer)
        .wrap(logger)
        .wrap(ThunkMiddleware)
        .build();

    let subscription = store.subscribe(|count: &i32| println!("Count changed: {count}"));
    store.dispatch(Action::Increment)?;
    store.dispatch(ActionWithLogLevel {
        action: Action::Increment,
        level: Level::Debug,
    })?;
    assert_eq!(store.get_state(), 2);

    let count = store
        .dispatch(thunk(|api| async move {
            // Nested thunks also receive the complete, typed dispatcher.
            api.dispatch(thunk(|api| async move {
                api.dispatch(ActionWithLogLevel {
                    action: Action::Set(10),
                    level: Level::Debug,
                })?;
                Ok::<_, DispatchError>(())
            }))
            .await?;
            Ok::<_, DispatchError>(api.get_state())
        }))
        .await?;
    assert_eq!(count, 10);
    subscription.unsubscribe();
    Ok(())
}
