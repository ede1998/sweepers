use core::fmt;
use std::{collections::VecDeque, vec};

use super::{Location, Minefield, State};

#[derive(Debug, Clone)]
pub struct Command {
    pub location: Location,
    pub action: Action,
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Reveal,
    Mark,
    Unmark,
    ToggleMark,
}

impl Command {
    pub fn new(location: impl Into<Location>, action: Action) -> Self {
        Self {
            location: location.into(),
            action,
        }
    }

    pub fn apply_recursively(&self, mf: &mut Minefield) -> Vec<Command> {
        let mut pending: VecDeque<_> = std::iter::once(self.clone()).collect();
        let mut cmds = vec![];
        while let Some(cmd) = pending.pop_front() {
            let result = cmd.apply(mf);
            if let Some(State::Revealed { adj_mines: 0 }) = result {
                let new_cmds = cmd
                    .location
                    .neighbours()
                    .map(|l| Command::new(l, Action::Reveal));
                pending.extend(new_cmds);
            }
            cmds.push(cmd);
        }
        cmds
    }

    pub fn apply(&self, mf: &mut Minefield) -> Option<State> {
        eprintln!(
            "Executing action {:?} at location {}",
            self.action, self.location
        );
        match self.action {
            Action::Reveal => mf.reveal(self.location),
            Action::Mark => mf.mark(self.location),
            Action::Unmark => mf.unmark(self.location),
            Action::ToggleMark => mf.toggle_mark(self.location),
        }
    }

    pub fn undo(&self, mf: &mut Minefield) -> Option<State> {
        match self.action {
            Action::Reveal => mf.unreveal(self.location),
            Action::Mark => mf.unmark(self.location),
            Action::Unmark => mf.mark(self.location),
            Action::ToggleMark => mf.toggle_mark(self.location),
        }
    }
}
