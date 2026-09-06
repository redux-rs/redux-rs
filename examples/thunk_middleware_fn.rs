use redux_rs::{
    DispatchError, Store,
    middlewares::thunk::{ThunkMiddleware, thunk},
};

#[derive(Debug)]
enum Action {
    Loaded(String),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), DispatchError> {
    let store = Store::builder(|_: String, action: &Action| match action {
        Action::Loaded(name) => name.clone(),
    })
    .wrap(ThunkMiddleware)
    .build();

    let name = String::from("Jane Doe");
    let length = store
        .dispatch(thunk(move |api| async move {
            tokio::task::yield_now().await; // Substitute an actual asynchronous request.
            api.dispatch(Action::Loaded(name))?;
            Ok::<_, DispatchError>(api.select(|name: &String| name.len()))
        }))
        .await?;

    assert_eq!(length, 8);
    assert_eq!(store.get_state(), "Jane Doe");
    Ok(())
}
