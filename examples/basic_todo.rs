// The todo-list state and actions from Redux's core concepts:
// https://redux.js.org/introduction/core-concepts/
// This example composes two reducers and uses both named and function selectors.

use redux_rs::{DispatchError, Selector, Store};

// JavaScript state shape:
// {
//   todos: [{ text: 'Eat food', completed: true },
//           { text: 'Exercise', completed: false }],
//   visibilityFilter: 'SHOW_COMPLETED'
// }
// Rust equivalent (the store starts with an empty list):
#[derive(Clone, Debug, Default)]
struct State {
    todos: Vec<Todo>,
    visibility_filter: VisibilityFilter,
}

#[derive(Clone, Debug)]
struct Todo {
    text: String,
    completed: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum VisibilityFilter {
    ShowAll,
    #[default]
    ShowCompleted,
}

#[derive(Debug)]
enum Action {
    AddTodo { text: String },
    ToggleTodo { index: usize },
    SetVisibilityFilter { filter: VisibilityFilter },
}

// Each reducer owns one slice of state and ignores unrelated actions.
fn todos_reducer(mut todos: Vec<Todo>, action: &Action) -> Vec<Todo> {
    match action {
        Action::AddTodo { text } => todos.push(Todo {
            text: text.clone(),
            completed: false,
        }),
        Action::ToggleTodo { index } => {
            if let Some(todo) = todos.get_mut(*index) {
                todo.completed = !todo.completed;
            }
        }
        Action::SetVisibilityFilter { .. } => {}
    }
    todos
}

fn visibility_filter_reducer(filter: VisibilityFilter, action: &Action) -> VisibilityFilter {
    match action {
        Action::SetVisibilityFilter { filter } => *filter,
        _ => filter,
    }
}

// The root reducer combines the slices. Both reducers borrow the same action;
// dispatch can still return that action to the caller without requiring Clone.
fn reducer(state: State, action: &Action) -> State {
    State {
        todos: todos_reducer(state.todos, action),
        visibility_filter: visibility_filter_reducer(state.visibility_filter, action),
    }
}

struct SelectNumberCompletedTodos;

impl Selector<State> for SelectNumberCompletedTodos {
    type Result = usize;

    fn select(self, state: &State) -> Self::Result {
        state.todos.iter().filter(|todo| todo.completed).count()
    }
}

// A function also works as a selector. Return owned text for display while
// borrowing, rather than cloning, the entire state during selection.
fn select_visible_todos(state: &State) -> Vec<String> {
    state
        .todos
        .iter()
        .filter(|todo| state.visibility_filter == VisibilityFilter::ShowAll || todo.completed)
        .map(|todo| todo.text.clone())
        .collect()
}

fn main() -> Result<(), DispatchError> {
    // The core store is synchronous and needs no Tokio runtime.
    let store = Store::new(reducer);
    // Keep this handle: dropping it would unsubscribe immediately.
    let subscription = store.subscribe(|state: &State| println!("New state: {state:?}"));

    println!(
        "Completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );
    assert_eq!(store.select(SelectNumberCompletedTodos), 0);

    // { type: 'ADD_TODO', text: 'Go to swimming pool' }
    store.dispatch(Action::AddTodo {
        text: "Go to swimming pool".into(),
    })?;
    println!(
        "Completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );
    assert_eq!(store.select(SelectNumberCompletedTodos), 0);
    // The default filter hides incomplete todos.
    assert!(store.select(select_visible_todos).is_empty());

    // { type: 'TOGGLE_TODO', index: 0 }
    store.dispatch(Action::ToggleTodo { index: 0 })?;
    println!(
        "Completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );
    assert_eq!(store.select(SelectNumberCompletedTodos), 1);
    assert_eq!(store.select(select_visible_todos), ["Go to swimming pool"]);

    store.dispatch(Action::AddTodo {
        text: "Read Redux docs".into(),
    })?;
    assert_eq!(store.select(select_visible_todos), ["Go to swimming pool"]);

    // { type: 'SET_VISIBILITY_FILTER', filter: 'SHOW_ALL' }
    store.dispatch(Action::SetVisibilityFilter {
        filter: VisibilityFilter::ShowAll,
    })?;
    let visible = store.select(select_visible_todos);
    println!("Visible tasks: {visible:?}");
    assert_eq!(visible, ["Go to swimming pool", "Read Redux docs"]);

    // Toggling again marks the first todo incomplete; filtering changes the view,
    // not the stored list. An invalid index is a harmless no-op.
    store.dispatch(Action::ToggleTodo { index: 0 })?;
    store.dispatch(Action::ToggleTodo { index: 99 })?;
    store.dispatch(Action::SetVisibilityFilter {
        filter: VisibilityFilter::ShowCompleted,
    })?;
    assert_eq!(store.select(SelectNumberCompletedTodos), 0);
    assert!(store.select(select_visible_todos).is_empty());
    assert_eq!(store.select(|state: &State| state.todos.len()), 2);

    subscription.unsubscribe();
    Ok(())
}
