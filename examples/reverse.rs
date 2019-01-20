use redux_rs::{Store, Subscription};

// A simple counter.
type State = i8;

// Increment and decrement actions for the counter.
enum Action {
    Increment,
    Decrement
}

// Reducer for the counter.
fn reducer(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => state + 1,
        Action::Decrement => state - 1
    }
}

// A sample middleware that reverses the action passed to the reducer.
fn reverse_middleware(_: &mut Store<State, Action>, action: Action) -> Option<Action> {
    match action {
        Action::Increment => Some(Action::Decrement),
        Action::Decrement => Some(Action::Increment)
    }
}

fn main() {
    // Create the store.
    let mut store = Store::new(reducer, 0);

    // Add the reversing middleware.
    store.add_middleware(reverse_middleware);

    // Define listener.
    let listener: Subscription<State> = |state: &State| {
        println!("Counter changed! New value: {}", state);
    };

    // Subscribe listener.
    store.subscribe(listener);

    // Dispatch actions.
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Decrement);
    store.dispatch(Action::Decrement);

    // Print final value.
    println!("Final value: {}", store.state());
}
