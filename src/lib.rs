use log::warn;
use std::any::Any;
use std::marker::PhantomData;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

type DynamicAction = Box<dyn Any + Send + 'static>;

pub trait Dispatch {
    fn dispatch(&self, action: DynamicAction);
}

pub trait WithState<State> {
    fn state(&self) -> State;
}

pub trait Reducer<State, Action> {
    fn reduce(&self, state: State, action: Action) -> State;
}

impl<F, State, Action> Reducer<State, Action> for F
where
    F: Fn(State, Action) -> State
{
    fn reduce(&self, state: State, action: Action) -> State {
        self(state, action)
    }
}

pub trait Subscriber<State>: Send {
    fn updated(&self, state: &State);
}

impl<F, State> Subscriber<State> for F
where
    F: for<'r> Fn(&'r State) + Send
{
    fn updated(&self, state: &State) {
        self(state)
    }
}

pub struct Store<State, Action> {
    data: Arc<Mutex<StoreData<State>>>,

    action_sender: Sender<DynamicAction>,

    _join_handle: JoinHandle<()>,
    _types: PhantomData<Action>
}

struct StoreData<State> {
    state: Option<State>,
    subscribers: Vec<Box<dyn Subscriber<State>>>
}

impl<State, Action> Store<State, Action> {
    pub fn new<RootReducer>(reducer: RootReducer) -> Self
    where
        Action: 'static,
        RootReducer: Reducer<State, Action> + Send + 'static,
        State: Default + Send + 'static
    {
        Self::new_with_state(reducer, State::default())
    }

    pub fn new_with_state<RootReducer>(reducer: RootReducer, default_state: State) -> Self
    where
        Action: 'static,
        RootReducer: Reducer<State, Action> + Send + 'static,
        State: Send + 'static
    {
        let (sender, receiver): (Sender<DynamicAction>, _) = channel();

        let data = Arc::new(Mutex::new(StoreData {
            state: Some(default_state),
            subscribers: Vec::new()
        }));

        let thread_data = data.clone();
        let join_handle = thread::spawn(move || {
            while let Ok(action) = receiver.recv() {
                let action = match action.downcast() {
                    Ok(action) => *action,
                    Err(_) => {
                        warn!("Action should be of the same type of the store. You're probably missing middleware to handle the kind of action you've send to the store!");
                        continue;
                    }
                };

                let mut data_lock = thread_data
                    .lock()
                    .expect("Only returns an error when a previous piece of code paniced");

                let old_state = data_lock.state.take().unwrap();
                let new_state = reducer.reduce(old_state, action);

                data_lock.state = Some(new_state);

                let state = data_lock.state.as_ref().unwrap();
                data_lock.subscribers.iter().for_each(|s| s.updated(state));
            }
        });

        let store = Self {
            data,

            action_sender: sender,

            _join_handle: join_handle,
            _types: Default::default()
        };

        store
    }

    pub fn subscribe(&mut self, subscriber: Box<dyn Subscriber<State>>) {
        self.data.lock().unwrap().subscribers.push(subscriber);
    }
}

impl<State, Action> Dispatch for Store<State, Action> {
    fn dispatch(&self, action: DynamicAction) {
        // This only fails when the receiver is gone (inner store) this should never happen
        let _ = self.action_sender.send(action);
    }
}

impl<State, Action> WithState<State> for Store<State, Action>
where
    State: Clone
{
    fn state(&self) -> State {
        self.data
            .lock()
            .expect("Only returns an error when a previous piece of code paniced")
            .state
            .as_ref()
            .expect(
                "State never contains None, only during updates, which are behind this very mutex"
            )
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Barrier
    };

    #[derive(Clone, Default, Debug, PartialEq)]
    struct CounterStore(i32);

    enum CounterAction {
        Increment,
        Decrement
    }

    fn counter_reducer(state: CounterStore, action: CounterAction) -> CounterStore {
        let value = state.0;

        let new_value = match action {
            CounterAction::Increment => value + 1,
            CounterAction::Decrement => value - 1
        };

        CounterStore(new_value)
    }

    #[test]
    fn counter_without_middleware() {
        let (sender, receiver) = channel();

        let mut store = Store::new(counter_reducer);
        store.subscribe(Box::new(move |state: &CounterStore| {
            sender.send(state.0).unwrap();
        }));

        store.dispatch(Box::new(CounterAction::Increment));
        store.dispatch(Box::new(CounterAction::Decrement));

        assert_eq!(1, receiver.recv().unwrap());
        assert_eq!(0, receiver.recv().unwrap());
    }

    #[test]
    fn counter_send_invalid_action() {
        let number_of_updates = Arc::new(AtomicU8::new(0));
        let incremented = Arc::new(Barrier::new(2));

        let mut store = Store::new(counter_reducer);

        let subscriber_number_of_updates = number_of_updates.clone();
        let subscriber_barrier = incremented.clone();
        store.subscribe(Box::new(move |state: &CounterStore| {
            subscriber_number_of_updates.fetch_add(1, Ordering::Relaxed);
            if state.0 == 1 {
                subscriber_barrier.wait();
            }
        }));

        store.dispatch(Box::new("test"));
        store.dispatch(Box::new(CounterAction::Increment));

        incremented.wait();
        assert_eq!(1, number_of_updates.load(Ordering::Relaxed));
    }
}
