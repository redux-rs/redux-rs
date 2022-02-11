pub trait Reducer<State, Action> {
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
