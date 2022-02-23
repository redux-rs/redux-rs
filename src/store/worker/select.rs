use crate::store::worker::Work;
use crate::Selector;
use std::marker::PhantomData;

pub struct Select<State, S>
where
    S: Selector<State>
{
    selector: S,
    _types: PhantomData<State>
}

impl<State, S> Select<State, S>
where
    S: Selector<State>
{
    pub fn new(selector: S) -> Self {
        Select {
            selector,
            _types: Default::default()
        }
    }

    pub fn into_selector(self) -> S {
        self.selector
    }
}

impl<State, S> Work for Select<State, S>
where
    State: Send,
    S: Selector<State> + Send,
    S::Result: Send
{
    type Result = S::Result;
}
