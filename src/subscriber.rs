pub trait Subscriber<State> {
    fn notify(&self, state: &State);
}

impl<F, State> Subscriber<State> for F
where
    F: Fn(&State)
{
    fn notify(&self, state: &State) {
        self(state);
    }
}
