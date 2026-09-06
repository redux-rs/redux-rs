use redux_rs::{DispatchError, Store};

#[derive(Clone, Debug, Default)]
struct State {
    todos: Vec<String>,
}

#[derive(Debug)]
enum Action {
    Add(String),
}

fn reducer(mut state: State, action: &Action) -> State {
    match action {
        Action::Add(text) => state.todos.push(text.clone()),
    }
    state
}

fn main() -> Result<(), DispatchError> {
    let store = Store::new(reducer);
    let _subscription = store.subscribe(|state: &State| println!("Todos: {:?}", state.todos));
    store.dispatch(Action::Add("Try synchronous Redux".into()))?;
    assert_eq!(store.select(|state: &State| state.todos.len()), 1);
    Ok(())
}
