/// Function signature for a reducer.
///
/// # Example
///
/// ```
/// # use redux_rs::Reducer;
/// #
/// enum Action {
///     Increment,
///     Decrement
/// }
///
/// let reducer: Reducer<u8, Action> = |state: &u8, action: &Action| -> u8 {
///     match action {
///         Action::Increment => state + 1,
///         Action::Decrement => state - 1
///     }
/// };
/// ```
pub type Reducer<State, Action> = fn(&State, &Action) -> State;

#[macro_export]
/// Combines multiple reducers into a single one.
///
/// The first one gets called first, chained into the second one and so on...
///
/// # Usage
///
/// ```
/// # use redux_rs::{combine_reducers, Reducer};
/// #
/// # type State = u8;
/// #
/// # type Action = bool;
/// #
/// # fn first_reducer(_: &State, _: &Action) -> State {
/// #     0
/// # }
/// #
/// # fn second_reducer(_: &State, _: &Action) -> State {
/// #     0
/// # }
/// #
/// # fn third_reducer(_: &State, _: &Action) -> State {
/// #     0
/// # }
/// #
/// let reducer: Reducer<State, Action> = combine_reducers!(State, &Action, first_reducer, second_reducer, third_reducer);
/// ```
/// (`State` and `Action` being the actual types.)
///
/// # Example
///
/// ```
/// # use redux_rs::{combine_reducers, Reducer};
/// #
/// enum Action {
///     Increment,
///     Decrement
/// }
///
/// fn counter_reducer(state: &u8, action: &Action) -> u8 {
///     match action {
///         Action::Increment => state + 1,
///         Action::Decrement => state - 1
///     }
/// }
///
/// fn add_two_reducer(state: &u8, _: &Action) -> u8 {
///     state + 2
/// }
///
/// fn main() {
///     let reducer: Reducer<u8, Action> = combine_reducers!(u8, &Action, counter_reducer, add_two_reducer);
/// }
/// ```
macro_rules! combine_reducers {
    ($state:ty, $action:ty, $reducer:ident) => ($reducer);
    ($state:ty, $action:ty, $first:ident, $($second:ident),+) => (
        |state: &$state, action: $action| -> $state {
            (combine_reducers!($state, $action, $($second),+))(&$first(state, action), action)
        }
    )
}
