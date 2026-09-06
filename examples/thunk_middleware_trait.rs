// Port of the original trait-based thunk example, keeping its example name.
// In 0.4, thunk(...) accepts FnOnce rather than an async Thunk trait. A reusable
// request struct exposes execute(self, api), adapted with a consuming closure.
// Run with: cargo run --example thunk_middleware_trait --features thunk

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
        UserAction::UsersLoaded { users } => UserState {
            users: users.clone(),
        },
    }
}

// A struct gives the operation a name and can carry per-request configuration.
// It need not implement Clone: execute consumes the request exactly once.
struct LoadUsersThunk {
    delay: Duration,
}

impl LoadUsersThunk {
    async fn execute<Inputs, Output, Path>(
        self,
        api: MiddlewareApi<UserState, Inputs, Output>,
    ) -> DispatchResult<usize>
    where
        Inputs: Promote<UserAction, Path>,
    {
        // Emulate a request to a user API without requiring network access.
        sleep(self.delay).await;
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
        Ok(api.select(|state: &UserState| state.users.len()))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), DispatchError> {
    let store = Store::builder(user_reducer).wrap(ThunkMiddleware).build();
    let subscription = store.subscribe(|state: &UserState| println!("Users: {:?}", state.users));
    assert!(store.select(|state: &UserState| state.users.is_empty()));

    let request = LoadUsersThunk {
        delay: Duration::from_millis(100),
    };
    // The closure moves the request into the thunk, so its owned configuration
    // remains available across await points. No async-trait or manual boxing.
    let count = store
        .dispatch(thunk(move |api| request.execute(api)))
        .await?;
    assert_eq!(count, 2);

    // Awaiting dispatch means both the request and its state update are done.
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
    subscription.unsubscribe();
    Ok(())
}
