use redux_rs::middlewares::thunk::{ThunkMiddleware, thunk};
use redux_rs::{DispatchError, DispatchResult, MiddlewareApi, Promote, Store};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Default, Debug, PartialEq)]
struct UserState {
    users: Vec<User>,
}

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u8,
    name: String,
}

enum UserAction {
    UsersLoaded { users: Vec<User> },
}

fn user_reducer(_state: UserState, action: &UserAction) -> UserState {
    match action {
        UserAction::UsersLoaded { users } => UserState {
            users: users.clone(),
        },
    }
}

struct LoadUsersThunk;
// Adapt the struct's method with thunk(...) instead of implementing the old async trait.
impl LoadUsersThunk {
    async fn execute<Inputs, Output, Path>(
        self,
        store_api: MiddlewareApi<UserState, Inputs, Output>,
    ) -> DispatchResult<()>
    where
        Inputs: Promote<UserAction, Path>,
    {
        // Emulate api call by delaying for 100 ms
        sleep(Duration::from_millis(100)).await;

        // Return the data to the store
        store_api.dispatch(UserAction::UsersLoaded {
            users: vec![
                User {
                    id: 0,
                    name: "John Doe".to_string(),
                },
                User {
                    id: 1,
                    name: "Jane Doe".to_string(),
                },
            ],
        })?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), DispatchError> {
    // Set up the store with a reducer and wrap it with thunk middleware
    // Wrap thunks with thunk(...); ordinary actions can still be dispatched directly
    let store = Store::builder(user_reducer).wrap(ThunkMiddleware).build();

    // Dispatch our thunk which emulates loading users from an api, and await completion
    store
        .dispatch(thunk(|store_api| LoadUsersThunk.execute(store_api)))
        .await?;

    // Get the users from the store
    let users = store.select(|state: &UserState| state.users.clone());
    assert_eq!(
        users,
        vec![
            User {
                id: 0,
                name: "John Doe".to_string(),
            },
            User {
                id: 1,
                name: "Jane Doe".to_string(),
            },
        ]
    );
    Ok(())
}
