pub trait Selector<State> {
    type Result;

    fn select(&self, state: &State) -> Self::Result;
}

impl<F, State, Result> Selector<State> for F
where
    F: Fn(&State) -> Result,
{
    type Result = Result;

    fn select(&self, state: &State) -> Self::Result {
        self(state)
    }
}
