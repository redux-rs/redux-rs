use std::marker::PhantomData;

pub trait Store<State, Action> {
    fn state(&self) -> &State;
    fn dispatch(&mut self, action: Action);

    fn subscribe<S>(&mut self, subscriber: S)
    where
        S: StoreSubscriber<State> + 'static;
}

pub trait StoreSubscriber<State> {
    fn updated(&mut self, state: &State);
}

impl<S, State> StoreSubscriber<State> for S
where
    S: FnMut(&State),
{
    fn updated(&mut self, state: &State) {
        self(state)
    }
}

pub trait Reducer<State, Action> {
    fn reduce(&self, state: State, action: Action) -> State;
}

impl<R, State, Action> Reducer<State, Action> for R
where
    R: Fn(State, Action) -> State,
{
    fn reduce(&self, state: State, action: Action) -> State {
        self(state, action)
    }
}

pub struct StoreBase<State, Action, Reducer>
where
    Reducer: crate::Reducer<State, Action>,
{
    state: Option<State>,
    subscribers: Vec<Box<dyn StoreSubscriber<State>>>,
    reducer: Reducer,
    _types: PhantomData<Action>,
}

impl<State, Action, Reducer> StoreBase<State, Action, Reducer>
where
    Reducer: crate::Reducer<State, Action>,
{
    pub fn with_default_state(reducer: Reducer) -> Self
    where
        State: Default,
    {
        StoreBase {
            state: Some(Default::default()),
            subscribers: Default::default(),
            reducer,
            _types: Default::default(),
        }
    }
}

impl<State, Action, Reducer> Store<State, Action> for StoreBase<State, Action, Reducer>
where
    Reducer: crate::Reducer<State, Action>,
{
    fn state(&self) -> &State {
        // We can always unwrap here,
        // state is an Option because I didn't want to use unsafe code in the dispatch method
        &self.state.as_ref().unwrap()
    }

    fn dispatch(&mut self, action: Action) {
        // We can always unwrap here, state is only None when calculating the new state
        // Since we have a mutual reference this won't cause any further issues
        let old_state = self.state.take().unwrap();
        let new_state = self.reducer.reduce(old_state, action);

        // Set the new calculated state
        self.state = Some(new_state);

        // Let all subscribers know that the store updated
        for subscriber in &mut self.subscribers {
            subscriber.updated(&self.state.as_ref().unwrap());
        }
    }

    fn subscribe<S>(&mut self, subscriber: S)
    where
        S: StoreSubscriber<State> + 'static,
    {
        self.subscribers.push(Box::new(subscriber));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    enum CounterAction {
        Up,
        Down,
    }

    #[derive(Default, Debug, PartialEq)]
    struct Counter {
        value: i32,
    }

    #[test]
    fn base_example() {
        let mut store =
            StoreBase::with_default_state(|state: Counter, action: CounterAction| match action {
                CounterAction::Up => Counter {
                    value: state.value + 1,
                    ..state
                },
                CounterAction::Down => Counter {
                    value: state.value - 1,
                    ..state
                },
            });

        let unit_test_value = Arc::new(AtomicI32::new(0));

        let unit_test_value_1 = unit_test_value.clone();
        store.subscribe(move |state: &Counter| {
            unit_test_value_1.fetch_add(state.value, Ordering::Relaxed);
        });

        let unit_test_value_2 = unit_test_value.clone();
        store.subscribe(move |state: &Counter| {
            unit_test_value_2.fetch_add(state.value, Ordering::Relaxed);
        });

        // Test initial state
        assert_eq!(&Counter { value: 0 }, store.state());

        // Count up
        store.dispatch(CounterAction::Up);
        assert_eq!(&Counter { value: 1 }, store.state());

        // Count up
        store.dispatch(CounterAction::Up);
        assert_eq!(&Counter { value: 2 }, store.state());

        // Count down
        store.dispatch(CounterAction::Down);
        assert_eq!(&Counter { value: 1 }, store.state());

        // Test subscriber state
        assert_eq!(8, unit_test_value.load(Ordering::Relaxed));
    }
}
