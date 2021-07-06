use super::Location;

#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub location: Location,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct ExecutedCommand {
    pub location: Location,
    pub action: Action,
    pub updated_locations: Vec<Location>,
}

impl ExecutedCommand {
    pub fn new(cmd: PendingCommand, updated_locations: Vec<Location>) -> Self {
        Self {
            location: cmd.location,
            action: cmd.action,
            updated_locations,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Reveal,
    Mark,
    Unmark,
    ToggleMark,
}

impl PendingCommand {
    pub fn new(location: impl Into<Location>, action: Action) -> Self {
        Self {
            location: location.into(),
            action,
        }
    }

    pub fn executed(self, updated_locations: Vec<Location>) -> ExecutedCommand {
        ExecutedCommand::new(self, updated_locations)
    }

    // pub fn undo(&self, mf: &mut Minefield) -> Option<State> {
    //     match self.action {
    //         Action::Reveal => mf.unreveal(self.location),
    //         Action::Mark => mf.unmark(self.location),
    //         Action::Unmark => mf.mark(self.location),
    //         Action::ToggleMark => mf.toggle_mark(self.location),
    //     }
    // }
}
