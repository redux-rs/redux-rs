use crate::{MiddleWare, StoreApi};
use async_trait::async_trait;
use std::sync::Arc;

pub struct ThunkMiddleware;

#[async_trait]
impl<State, Action, Inner> MiddleWare<State, ActionOrThunk<State, Action, Inner>, Inner, Action> for ThunkMiddleware
where
    Action: Send + 'static,
    State: Send + 'static,
    Inner: StoreApi<State, Action> + Send + Sync + 'static,
{
    async fn dispatch(&self, action: ActionOrThunk<State, Action, Inner>, inner: &Arc<Inner>) {
        match action {
            ActionOrThunk::Action(action) => {
                inner.dispatch(action).await;
            }
            ActionOrThunk::Thunk(thunk) => {
                let api = inner.to_owned();

                tokio::spawn(async move {
                    thunk.execute(api).await;
                });
            }
        }
    }
}

pub enum ActionOrThunk<State, Action, Api>
where
    Action: Send + 'static,
    State: Send + 'static,
    Api: StoreApi<State, Action> + Send + Sync,
{
    Action(Action),
    Thunk(Box<dyn Thunk<State, Action, Api> + Send + Sync>),
}

#[async_trait]
pub trait Thunk<State, Action, Api>
where
    Action: Send + 'static,
    State: Send + 'static,
    Api: StoreApi<State, Action> + Send + Sync + 'static,
{
    async fn execute(&self, store_api: Arc<Api>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
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

    fn user_reducer(state: UserState, action: UserAction) -> UserState {
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

    #[tokio::test]
    async fn load_users_thunk() {
        let store = Store::new(user_reducer).wrap(ThunkMiddleware).await;
        store.dispatch(ActionOrThunk::Thunk(Box::new(LoadUsersThunk))).await;

        let users = store.select(|state: &UserState| state.users.clone()).await;
        assert_eq!(users, vec![]);

        sleep(Duration::from_millis(200)).await;

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
}
