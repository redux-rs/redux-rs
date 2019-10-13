/// Function signature for a subscription.
///
/// A Subscription will be called, whenever an action is dispatched (and reaches the reducer).
/// It receives a reference to the current state, which might or might not be used.
///
/// # Example
///
/// ```
/// # use redux_rs::{Store};
/// #
/// # type State = u8;
/// # let initial_state = 0;
/// #
/// # fn reducer(_: &State, action: &bool) -> State {
/// #     0
/// # }
/// #
/// let mut store = Store::new(reducer, initial_state);
///
/// let listener = |state: &State| {
///     println!("Something changed! New value: {}", state);
/// };
///
/// store.subscribe(listener);
/// ```
pub type Subscription<State> = dyn Fn(&State);
