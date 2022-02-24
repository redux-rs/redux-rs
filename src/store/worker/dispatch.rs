use crate::store::worker::Work;

pub struct Dispatch<Action>
where
    Action: Send,
{
    action: Action,
}

impl<Action> Dispatch<Action>
where
    Action: Send,
{
    pub fn new(action: Action) -> Self {
        Dispatch { action }
    }

    pub fn into_action(self) -> Action {
        self.action
    }
}

impl<Action> Work for Dispatch<Action>
where
    Action: Send,
{
    type Result = ();
}
