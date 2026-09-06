// A reusable async function that loads users and dispatches the response.
// Run with: cargo run --example thunk_middleware_fn --features thunk

use redux_rs::{
    DispatchError, DispatchResult, MiddlewareApi, Promote, Store,
    middlewares::thunk::{ThunkMiddleware, thunk},
};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, Debug, Default, PartialEq)]
struct UserState {
    users: Vec<User>,
}

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u8,
    name: String,
}

#[derive(Debug)]
enum UserAction {
    UsersLoaded { users: Vec<User> },
}

fn user_reducer(_state: UserState, action: &UserAction) -> UserState {
    match action {
        // The reducer borrows the action; copy its payload into the new state.
        UserAction::UsersLoaded { users } => UserState {
            users: users.clone(),
        },
    }
}

// Promote expresses that this API accepts UserAction, even through wrappers.
// Inputs, Output, and the conversion Path are inferred when dispatching the thunk.
async fn load_users<Inputs, Output, Path>(
    api: MiddlewareApi<UserState, Inputs, Output>,
) -> DispatchResult<usize>
where
    Inputs: Promote<UserAction, Path>,
{
    // Emulate an asynchronous API request. No network access is needed to run
    // this example; replace this delay and fixture with your HTTP client.
    sleep(Duration::from_millis(100)).await;
    api.dispatch(UserAction::UsersLoaded {
        users: vec![
            User {
                id: 0,
                name: "John Doe".into(),
            },
            User {
                id: 1,
                name: "Jane Doe".into(),
            },
        ],
    })?;

    // A thunk can return an application value as well as dispatch actions.
    Ok(api.select(|state: &UserState| state.users.len()))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), DispatchError> {
    // Build the complete middleware chain before dispatching anything.
    let store = Store::builder(user_reducer).wrap(ThunkMiddleware).build();
    let subscription = store.subscribe(|state: &UserState| println!("Users: {:?}", state.users));
    assert!(store.select(|state: &UserState| state.users.is_empty()));

    // Await the request's completion and propagate errors to main. The result
    // is available directly; there is no detached task to wait for separately.
    let count = store.dispatch(thunk(load_users)).await?;
    assert_eq!(count, 2);

    // Read the full response and verify both user IDs and names.
    let users = store.select(|state: &UserState| state.users.clone());
    assert_eq!(
        users,
        vec![
            User {
                id: 0,
                name: "John Doe".into()
            },
            User {
                id: 1,
                name: "Jane Doe".into()
            },
        ]
    );

    // Thunk middleware preserves ordinary action dispatch; no ActionOrThunk
    // envelope is needed for either the request or its response actions.
    store.dispatch(UserAction::UsersLoaded { users: Vec::new() })?;
    assert!(store.get_state().users.is_empty());
    subscription.unsubscribe();
    Ok(())
}
