// Implementation of a very basic todo list described on: https://redux.js.org/introduction/core-concepts/
// This example shows how to update and select state

use redux_rs::{DispatchError, Selector, Store};

// Javascript state:
//
// {
//   todos: [{
//     text: 'Eat food',
//     completed: true
//   }, {
//     text: 'Exercise',
//     completed: false
//   }],
//   visibilityFilter: 'SHOW_COMPLETED'
// }
//
// Rest equivalent:
#[derive(Default, Debug, Clone)]
struct State {
    todos: Vec<Todo>,
    visibility_filter: VisibilityFilter,
}

#[derive(Debug, Clone)]
struct Todo {
    text: String,
    completed: bool,
}

#[derive(Debug, Clone, Default)]
enum VisibilityFilter {
    ShowAll,
    #[default]
    ShowCompleted,
}

enum Action {
    AddTodo { text: String },
    ToggleTodo { index: usize },
    SetVisibilityFilter { filter: VisibilityFilter },
}

fn reducer(mut state: State, action: &Action) -> State {
    match action {
        Action::AddTodo { text } => State {
            todos: {
                state.todos.push(Todo {
                    text: text.clone(),
                    completed: false,
                });
                state.todos
            },
            ..state
        },
        Action::ToggleTodo { index } => State {
            todos: {
                if let Some(todo) = state.todos.get_mut(*index) {
                    todo.completed = !todo.completed;
                }
                state.todos
            },
            ..state
        },
        Action::SetVisibilityFilter { filter } => State {
            visibility_filter: filter.clone(),
            ..state
        },
    }
}

struct SelectNumberCompletedTodos;
impl Selector<State> for SelectNumberCompletedTodos {
    type Result = usize;

    fn select(self, state: &State) -> Self::Result {
        state.todos.iter().filter(|t| t.completed).count()
    }
}

fn main() -> Result<(), DispatchError> {
    let store = Store::new(reducer);
    // Keep the handle alive to stay subscribed.
    let _subscription = store.subscribe(|state: &State| println!("New state: {state:?}"));

    // Print number of completed tasks
    println!(
        "Number of completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );

    // { type: 'ADD_TODO', text: 'Go to swimming pool' }
    store.dispatch(Action::AddTodo {
        text: "Go to swimming pool".to_string(),
    })?;

    // Print number of completed tasks
    println!(
        "Number of completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );

    // { type: 'TOGGLE_TODO', index: 0 }
    store.dispatch(Action::ToggleTodo { index: 0 })?;

    // Print number of completed tasks
    println!(
        "Number of completed tasks: {}",
        store.select(SelectNumberCompletedTodos)
    );

    // { type: 'SET_VISIBILITY_FILTER', filter: 'SHOW_ALL' }
    store.dispatch(Action::SetVisibilityFilter {
        filter: VisibilityFilter::ShowAll,
    })?;
    assert_eq!(store.select(SelectNumberCompletedTodos), 1);
    assert!(store.select(|state: &State| state.todos[0].text == "Go to swimming pool"));
    assert!(
        store.select(|state: &State| matches!(state.visibility_filter, VisibilityFilter::ShowAll))
    );
    Ok(())
}
