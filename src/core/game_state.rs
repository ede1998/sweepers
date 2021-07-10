use std::time::{Duration, Instant};

use super::{Area, GroundKind, State};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Initial { mine_count: usize },
    InProgress { start_time: Instant },
    Loss { game_duration: Duration },
    Win { game_duration: Duration },
}

impl GameState {
    pub fn new(mine_count: usize) -> Self {
        Self::Initial { mine_count }
    }

    pub(super) fn update(&mut self, fog: &Area<State>, ground: &Area<GroundKind>) -> bool {
        let lost = fog.iter().any(State::is_exploded);
        let won = fog
            .iter()
            .zip(ground.iter())
            .all(|(&s, &g)| s.is_revealed() ^ (g.is_mine() && s == State::Marked));
        let new = match self {
            GameState::Initial { .. } => match fog.iter().all(State::is_hidden) {
                true => self.clone(),
                false => match lost {
                    true => GameState::Loss {
                        game_duration: Duration::ZERO,
                    },
                    false => GameState::InProgress {
                        start_time: Instant::now(),
                    },
                },
            },
            GameState::InProgress { start_time } => match (won, lost) {
                (false, true) => GameState::Loss {
                    game_duration: start_time.elapsed(),
                },
                (true, false) => GameState::Win {
                    game_duration: start_time.elapsed(),
                },
                (false, false) => self.clone(),
                (true, true) => {
                    panic!("Invalid transition, both win and lose at the same time.")
                }
            },
            _ => self.clone(),
        };
        let old = std::mem::replace(self, new);
        *self != old
    }

    /// Returns `true` if the game_state is [`Win`].
    pub fn is_win(&self) -> bool {
        matches!(self, Self::Win { .. })
    }

    /// Returns `true` if the game_state is [`Loss`].
    pub fn is_loss(&self) -> bool {
        matches!(self, Self::Loss { .. })
    }
}
