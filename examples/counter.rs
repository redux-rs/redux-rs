use redux_rs::{Store};

#[derive(Default)]
// This is a state. It describes an immutable object.
// It is changed via a 'reducer', a function which receives an action and returns a new state modified based on the action.
struct State {
    counter: i8
}

// The actions describe what the reducer has to do.
// Rust enums can carry a payload, which one can use to pass some value to the reducer.
enum Action {
    Increment,
    Decrement
}

// Here comes the reducer. It gets the current state plus an action to perform and returns a new state.
fn counter_reducer(state: &State, action: &Action) -> State {
    match action {
        Action::Increment => State {
            counter: state.counter + 1
        },
        Action::Decrement => State {
            counter: state.counter - 1
        }
    }
}

fn main() {
    // A store is a way to handle a state. It gets created once and after that it can be read and changed via dispatching actions.
    let mut store = Store::new(counter_reducer, State::default());

    // A listener getting triggered whenever the state changes.
    let listener = |state: &State| {
        println!("Counter changed! New value: {}", state.counter);
    };

    // Listener gets subscribed to the store.
    store.subscribe(listener);

    // Now, let's dispatch some actions!
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Increment);
    store.dispatch(Action::Decrement);
    store.dispatch(Action::Decrement);

    // Retrieve the value at any time.
    println!("Final value: {}", store.state().counter);
}
