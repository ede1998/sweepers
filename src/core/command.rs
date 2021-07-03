use super::{Location, Minefield};

pub struct Command {
    pub location: Location,
    pub action: Action,
}

pub enum Action {
    Reveal,
    Mark,
}

impl Command {
    pub fn new(location: impl Into<Location>, action: Action) -> Self {
        Self {
            location: location.into(),
            action,
        }
    }

    pub fn apply(&self, mf: &mut Minefield) {
        mf.reveal(self.location);
    }
}
