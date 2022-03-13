use async_trait::async_trait;
use redux_rs::middlewares::thunk::{ActionOrThunk, Thunk, ThunkMiddleware};
use redux_rs::{Store, StoreApi};
use std::sync::Arc;
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

fn user_reducer(_state: UserState, action: UserAction) -> UserState {
    match action {
        UserAction::UsersLoaded { users } => UserState { users },
    }
}

struct LoadUsersThunk;
#[async_trait]
impl<Api> Thunk<UserState, UserAction, Api> for LoadUsersThunk
where
    Api: StoreApi<UserState, UserAction> + Send + Sync + 'static,
{
    async fn execute(&self, store_api: Arc<Api>) {
        // Emulate api call by delaying for 100 ms
        sleep(Duration::from_millis(100)).await;

        // Return the data to the store
        store_api
            .dispatch(UserAction::UsersLoaded {
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
            })
            .await;
    }
}

#[tokio::main]
async fn main() {
    // Set up the store with a reducer and wrap it with thunk middleware
    // Because the store is now wrapped with ThunkMiddleware we need to dispatch ActionOrThunk instead of actions
    let store = Store::new(user_reducer).wrap(ThunkMiddleware).await;

    // Dispatch our thunk which emulates loading users from an api
    store.dispatch(ActionOrThunk::Thunk(Box::new(LoadUsersThunk))).await;

    // Wait till the "api call" is completed
    sleep(Duration::from_millis(200)).await;

    // Get the users from the store
    let users = store.select(|state: &UserState| state.users.clone()).await;
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
}
