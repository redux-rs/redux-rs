use redux_rs::{Selector, Store};

struct Counter(i8);

enum Action {
    Increment,
    Decrement
}

fn reducer(state: Counter, action: Action) -> Counter {
    let current_value = state.0;

    match action {
        Action::Increment => Counter(current_value + 1),
        Action::Decrement => Counter(current_value - 1)
    }
}

fn value_selector(store: &Counter) -> i8 {
    store.0
}

struct ValueSelector;
impl Selector<Counter> for ValueSelector {
    type Result = i8;

    fn select(&self, state: &Counter) -> Self::Result {
        state.0
    }
}

#[tokio::test]
async fn fn_selector() {
    // Create a new store with default value 42
    let store = Store::new_with_state(reducer, Counter(42));

    // Verify that the current value is 42
    assert_eq!(store.select(value_selector).await, 42);

    // Dispatch an increment action, the new value should be 43
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(value_selector).await, 43);

    // Dispatch another increment action, the new value should be 44
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(value_selector).await, 44);

    // Dispatch a decrement action, the new value should be 43
    store.dispatch(Action::Decrement).await;
    assert_eq!(store.select(value_selector).await, 43);
}

#[tokio::test]
async fn closure_selector() {
    // Create a new store with default value 42
    let store = Store::new_with_state(reducer, Counter(42));

    let closure_selector = |counter: &Counter| counter.0;

    // Verify that the current value is 42
    assert_eq!(store.select(closure_selector).await, 42);

    // Dispatch an increment action, the new value should be 43
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(closure_selector).await, 43);

    // Dispatch another increment action, the new value should be 44
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(closure_selector).await, 44);

    // Dispatch a decrement action, the new value should be 43
    store.dispatch(Action::Decrement).await;
    assert_eq!(store.select(closure_selector).await, 43);
}

#[tokio::test]
async fn trait_selector() {
    // Create a new store with default value 42
    let store = Store::new_with_state(reducer, Counter(42));

    // Verify that the current value is 42
    assert_eq!(store.select(ValueSelector).await, 42);

    // Dispatch an increment action, the new value should be 43
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(ValueSelector).await, 43);

    // Dispatch another increment action, the new value should be 44
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(ValueSelector).await, 44);

    // Dispatch a decrement action, the new value should be 43
    store.dispatch(Action::Decrement).await;
    assert_eq!(store.select(ValueSelector).await, 43);
}

#[tokio::test]
async fn subscribe_to_updates() {
    // Create a new store with default value 42
    let store = Store::new_with_state(reducer, Counter(42));

    // Subscribe to every update and print the value
    store
        .subscribe(|store: &Counter| println!("New store value: {}", store.0))
        .await;

    // Verify that the current value is 42
    assert_eq!(store.select(ValueSelector).await, 42);

    // Dispatch an increment action, the new value should be 43
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(ValueSelector).await, 43);

    // Dispatch another increment action, the new value should be 44
    store.dispatch(Action::Increment).await;
    assert_eq!(store.select(ValueSelector).await, 44);

    // Dispatch a decrement action, the new value should be 43
    store.dispatch(Action::Decrement).await;
    assert_eq!(store.select(ValueSelector).await, 43);
}
