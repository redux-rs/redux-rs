// Implementation of a very basic todo list described on: https://redux.js.org/introduction/core-concepts/
// This example shows how to combine multiple reducers

use redux_rs::{Selector, Store};

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
#[derive(Default, Debug)]
struct State {
    todos: Vec<Todo>,
    visibility_filter: VisibilityFilter,
}

#[derive(Debug)]
struct Todo {
    text: String,
    completed: bool,
}

#[derive(Debug)]
enum VisibilityFilter {
    ShowAll,
    ShowCompleted,
}

impl Default for VisibilityFilter {
    fn default() -> Self {
        VisibilityFilter::ShowCompleted
    }
}

enum Action {
    AddTodo { text: String },
    ToggleTodo { index: usize },
    SetVisibilityFilter { filter: VisibilityFilter },
}

fn reducer(mut state: State, action: Action) -> State {
    match action {
        Action::AddTodo { text } => State {
            todos: {
                state.todos.push(Todo { text, completed: false });
                state.todos
            },
            ..state
        },
        Action::ToggleTodo { index } => State {
            todos: {
                if let Some(todo) = state.todos.get_mut(index) {
                    todo.completed = !todo.completed;
                }
                state.todos
            },
            ..state
        },
        Action::SetVisibilityFilter { filter } => State {
            visibility_filter: filter,
            ..state
        },
    }
}

struct SelectNumberCompletedTodos;
impl Selector<State> for SelectNumberCompletedTodos {
    type Result = usize;

    fn select(&self, state: &State) -> Self::Result {
        state.todos.iter().filter(|t| t.completed).count()
    }
}

#[tokio::main]
async fn main() {
    let store = Store::new(reducer);
    store.subscribe(|state: &State| println!("New state: {:?}", state)).await;

    // Print number of completed tasks
    println!("Number of completed tasks: {}", store.select(SelectNumberCompletedTodos).await);

    // { type: 'ADD_TODO', text: 'Go to swimming pool' }
    store
        .dispatch(Action::AddTodo {
            text: "Go to swimming pool".to_string(),
        })
        .await;

    // Print number of completed tasks
    println!("Number of completed tasks: {}", store.select(SelectNumberCompletedTodos).await);

    // { type: 'TOGGLE_TODO', index: 0 }
    store.dispatch(Action::ToggleTodo { index: 0 }).await;

    // Print number of completed tasks
    println!("Number of completed tasks: {}", store.select(SelectNumberCompletedTodos).await);

    // { type: 'SET_VISIBILITY_FILTER', filter: 'SHOW_ALL' }
    store
        .dispatch(Action::SetVisibilityFilter {
            filter: VisibilityFilter::ShowAll,
        })
        .await;
}
