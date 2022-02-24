/// # Reducer trait
/// A reducer is responsible to calculate the next state based on the current state and an action.
/// You can do this by implementing the `Reducer` or a function with the signature `Fn(State, Action) -> State`
///
/// ## Trait example
/// ```
/// use redux_rs::Reducer;
///
/// enum Action {
///     Increment,
///     Decrement,
/// }
///
/// impl Reducer<u8, Action> for u8 {
///     fn reduce(&self, state: u8, action: Action) -> u8 {
///         match action {
///             Action::Increment => state + 1,
///             Action::Decrement => state - 1,
///         }
///     }
/// }
/// ```
///
/// ## Fn example
/// ```
/// use redux_rs::Reducer;
///
/// enum Action {
///     Increment,
///     Decrement,
/// }
///
/// fn reduce(state: u8, action: Action) -> u8 {
///     match action {
///         Action::Increment => state + 1,
///         Action::Decrement => state - 1,
///     }
/// }
/// ```
pub trait Reducer<State, Action> {
    /// Method gets called every time a user dispatches an action to the store.
    /// This method takes the previous state and the action and is supposed to calculate the new state.
    fn reduce(&self, state: State, action: Action) -> State;
}

impl<F, State, Action> Reducer<State, Action> for F
where
    F: Fn(State, Action) -> State,
{
    fn reduce(&self, state: State, action: Action) -> State {
        self(state, action)
    }
}
