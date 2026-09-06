use std::cell::RefCell;

/// Receives a state snapshot after dispatch. See also [`crate::Store::listen`]
/// for Redux-style listeners that do not require cloning state.
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

/// Retain this handle to keep receiving notifications. Dropping it unsubscribes.
#[must_use = "dropping the subscription immediately unsubscribes"]
pub struct Subscription {
    cancel: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl Subscription {
    pub(crate) fn new(cancel: impl FnOnce() + 'static) -> Self {
        Self {
            cancel: RefCell::new(Some(Box::new(cancel))),
        }
    }

    /// Remove this listener. Repeated calls are harmless.
    pub fn unsubscribe(&self) {
        let cancel = self.cancel.borrow_mut().take();
        if let Some(cancel) = cancel {
            cancel();
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}
