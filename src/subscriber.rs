/// # Subscriber trait
/// A subscriber is what gets called every time a new state is calculated.
/// You create a subscriber by implementing the `Subscriber` trait or by creating a function with the signature `Fn(&State)`
///
/// ## Trait example
/// ```
/// use redux_rs::Subscriber;
///
/// #[derive(Debug)]
/// struct Counter(i8);
///
/// struct PrintSubscriber;
/// impl Subscriber<Counter> for PrintSubscriber {
///     fn notify(&self, state: &Counter) {
///         println!("State changed: {:?}", state);
///     }
/// }
/// ```
///
/// ## Fn example
/// ```
/// use redux_rs::{Store, Subscriber};
///
/// #[derive(Debug)]
/// struct Counter(i8);
///
/// fn print_subscriber(state: &Counter) {
///     println!("State changed: {:?}", state);
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn async_test() {
/// # let store = Store::new_with_state(|store: Counter, _action: ()| store, Counter(0));
/// # store.subscribe(print_subscriber).await;
/// # }
/// ```
pub trait Subscriber<State> {
    fn notify(&self, state: &State);
}

impl<F, State> Subscriber<State> for F
where
    F: Fn(&State),
{
    fn notify(&self, state: &State) {
        self(state);
    }
}
