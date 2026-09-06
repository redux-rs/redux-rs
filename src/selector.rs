/// A one-shot read of the state. Functions and consuming closures work directly.
pub trait Selector<State> {
    type Result;
    fn select(self, state: &State) -> Self::Result;
}

impl<F, State, Result> Selector<State> for F
where
    F: FnOnce(&State) -> Result,
{
    type Result = Result;
    fn select(self, state: &State) -> Self::Result {
        self(state)
    }
}
