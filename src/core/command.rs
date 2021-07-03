use super::{Location, Minefield};

pub struct Command {
    pub location: Location,
    pub action: Action,
}

pub enum Action {
    Reveal,
    Mark,
    Unmark,
}

impl Command {
    pub fn new(location: impl Into<Location>, action: Action) -> Self {
        Self {
            location: location.into(),
            action,
        }
    }

    pub fn apply(&self, mf: &mut Minefield) {
        match self.action {
            Action::Reveal => mf.reveal(self.location),
            Action::Mark => mf.mark(self.location),
            Action::Unmark => mf.unmark(self.location),
        }
    }

    pub fn undo(&self, mf: &mut Minefield) {
        match self.action {
            Action::Reveal => mf.unreveal(self.location),
            Action::Mark => mf.unmark(self.location),
            Action::Unmark => mf.mark(self.location),
        }
    }
}
