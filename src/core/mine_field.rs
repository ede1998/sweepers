use std::{collections::VecDeque, convert::TryInto, fmt, iter, time::Instant};

use crate::generator::{DummyGenerator, ImprovedGenerator};

use super::{Action, Area, ExecutedCommand, GameState, Location, PendingCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundKind {
    Mine,
    Dirt,
}

impl GroundKind {
    /// Returns `true` if the ground_kind is [`Dirt`].
    pub fn is_dirt(&self) -> bool {
        matches!(self, Self::Dirt)
    }

    /// Returns `true` if the ground_kind is [`Mine`].
    pub fn is_mine(&self) -> bool {
        matches!(self, Self::Mine)
    }
}

impl Default for GroundKind {
    fn default() -> Self {
        Self::Dirt
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Hidden,
    Marked,
    Revealed { adj_mines: usize },
    Exploded,
}

impl State {
    /// Returns `true` if the state is [`Exploded`].
    pub fn is_exploded(&self) -> bool {
        matches!(self, Self::Exploded)
    }

    /// Returns `true` if the state is [`Revealed`].
    pub fn is_revealed(&self) -> bool {
        matches!(self, Self::Revealed { .. })
    }

    /// Returns `true` if the state is [`Hidden`].
    pub fn is_hidden(&self) -> bool {
        matches!(self, Self::Hidden)
    }

    /// Returns `true` if the state is [`Marked`].
    pub fn is_marked(&self) -> bool {
        matches!(self, Self::Marked)
    }

    pub fn as_mut_revealed(&mut self) -> Option<&mut usize> {
        if let Self::Revealed { adj_mines } = self {
            Some(adj_mines)
        } else {
            None
        }
    }

    pub fn as_revealed(&self) -> Option<&usize> {
        if let Self::Revealed { adj_mines } = self {
            Some(adj_mines)
        } else {
            None
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Hidden
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            State::Hidden => write!(f, " "),
            State::Marked => write!(f, "F"),
            State::Exploded => write!(f, "B"),
            State::Revealed { adj_mines } => write!(f, "{}", adj_mines),
        }
    }
}

pub enum ExecutionResult {
    Failed,
    SuccessAndStateChange(ExecutedCommand),
    SuccessNoStateChange(ExecutedCommand),
}

pub struct Parameters {
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
}

impl Parameters {
    pub fn new(width: usize, height: usize, mine_count: usize) -> Self {
        Self {
            width,
            height,
            mine_count,
        }
    }
}

pub trait MinefieldGenerator {
    fn generate(&mut self, params: Parameters, not_a_mine: Location) -> Area<GroundKind>;
}

pub struct Minefield {
    ground: Area<GroundKind>,
    fog: Area<State>,
    state: GameState,
    generator: Box<dyn MinefieldGenerator>,
}

impl Minefield {
    pub fn new(params: Parameters) -> Self {
        Self {
            ground: Default::default(),
            fog: Area::new(params.width, params.height),
            state: GameState::new(params.mine_count),
            generator: Box::new(ImprovedGenerator),
        }
    }

    pub fn with_generator(params: Parameters, generator: Box<dyn MinefieldGenerator>) -> Self {
        Self {
            ground: Default::default(),
            fog: Area::new(params.width, params.height),
            state: GameState::new(params.mine_count),
            generator,
        }
    }

    /// Load an active game from the given string.
    /// # Cell types:
    /// * m   = hidden mine
    /// * M   = revealed mine
    /// * F   = marked with mine beneath
    /// * f   = marked without mine beneath
    /// * e   = hidden dirt
    /// * E   = revealed dirt
    /// * 0-8 = revealed dirt with unchecked mine count for readibility
    /// * \n  = new row
    pub fn new_active_game(grid: &str) -> Self {
        let width = grid.lines().next().unwrap().len();
        let height = grid.lines().count();

        let stripped_grid = grid.replace(|c: char| c.is_ascii_whitespace(), "");
        let ground = stripped_grid
            .chars()
            .map(|c| match c {
                c if "mMF".contains(c) => GroundKind::Mine,
                c if "efE012345678".contains(c) => GroundKind::Dirt,
                c => panic!("Invalid character {:?} cannot be interpreted as ground.", c),
            })
            .collect();
        let ground = Area::with_area(width, height, ground);

        let fog = stripped_grid
            .char_indices()
            .map(|(i, c)| match c {
                'M' => State::Exploded,
                c if "me".contains(c) => State::Hidden,
                c if "Ff".contains(c) => State::Marked,
                c if "E012345678".contains(c) => {
                    let expected = c.to_digit(10).and_then(|n| n.try_into().ok());
                    let location = Location::from_index(i, width);
                    let actual = Self::mines_in_proximity(&ground, location);
                    if let Some(expected) = expected {
                        if actual != expected {
                            eprintln!("Expected mine count {} in grid string differs from actual value {} at position {}.", expected, actual, location);
                        }
                    }
                    State::Revealed { adj_mines: actual }
                }
                o => panic!("Invalid character {} cannot be interpreted as fog.", o),
            })
            .collect();

        Self {
            ground,
            fog: Area::with_area(width, height, fog),
            state: GameState::InProgress {
                start_time: Instant::now(),
            },
            generator: Box::new(DummyGenerator),
        }
    }

    pub fn fog(&self) -> &Area<State> {
        &self.fog
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn width(&self) -> usize {
        self.fog.width()
    }

    pub fn height(&self) -> usize {
        self.fog.height()
    }

    pub fn mine_count(&self) -> usize {
        self.ground.iter().filter(|g| g.is_mine()).count()
    }

    pub fn mark_count(&self) -> usize {
        self.fog.iter().filter(|g| g.is_marked()).count()
    }

    pub fn reset(&mut self) {
        let (width, height, mine_count) = (self.width(), self.height(), self.mine_count());
        self.ground = Default::default();
        self.fog = Area::new(width, height);
        self.state = GameState::new(mine_count);
    }

    pub fn reveal_all(&mut self) {
        let Minefield { ground, fog, .. } = self;
        for (index, (s, &g)) in fog.iter_mut().zip(ground.iter()).enumerate() {
            let location = Location::from_index(index, ground.width());
            match g {
                GroundKind::Mine => *s = State::Exploded,
                GroundKind::Dirt => {
                    *s = State::Revealed {
                        adj_mines: Self::mines_in_proximity(ground, location),
                    }
                }
            }
        }
    }

    fn reveal_location(
        fog: &mut Area<State>,
        ground: &Area<GroundKind>,
        location: Location,
    ) -> Vec<Location> {
        let mut pending: VecDeque<_> = std::iter::once(location).collect();
        let mut affected = vec![];

        while let Some(current) = pending.pop_front() {
            let state = match fog.get_mut(current) {
                Some(state @ State::Hidden) => state,
                _ => continue,
            };

            let target_state = match ground.get(current) {
                Some(GroundKind::Dirt) => State::Revealed {
                    adj_mines: Self::mines_in_proximity(ground, current),
                },
                Some(GroundKind::Mine) => State::Exploded,
                None => continue,
            };

            *state = target_state;
            affected.push(current);

            if let State::Revealed { adj_mines: 0 } = target_state {
                pending.extend(current.neighbours());
            }
        }

        affected
    }

    fn mines_in_proximity(ground: &Area<GroundKind>, location: Location) -> usize {
        location
            .neighbours()
            .filter_map(|l| ground.get(l).copied())
            .filter(GroundKind::is_mine)
            .count()
    }

    pub fn unreveal(&mut self, location: Location) -> Option<State> {
        let s = self
            .fog
            .get_mut(location)
            .filter(|s| s.is_hidden() || s.is_revealed())?;
        *s = State::Hidden;
        Some(*s)
    }

    pub fn execute(&mut self, cmd: PendingCommand) -> ExecutionResult {
        eprintln!(
            "Executing action {:?} at location {}",
            cmd.action, cmd.location
        );
        let Minefield {
            ground,
            fog,
            state,
            generator,
        } = self;

        if let GameState::Initial { mine_count } = *state {
            let params = Parameters::new(fog.width(), fog.height(), mine_count);
            *ground = generator.generate(params, cmd.location);
        }

        let mut updated_locations = vec![cmd.location];
        match (cmd.action, fog.get_mut(cmd.location)) {
            (Action::Reveal, Some(State::Hidden)) => {
                updated_locations = Self::reveal_location(fog, ground, cmd.location);
            }
            (Action::ToggleMark | Action::Mark, Some(s @ State::Hidden)) => *s = State::Marked,
            (Action::ToggleMark | Action::Unmark, Some(s @ State::Marked)) => *s = State::Hidden,
            _ => return ExecutionResult::Failed,
        }

        match state.update(fog, ground) {
            true => ExecutionResult::SuccessAndStateChange(cmd.executed(updated_locations)),
            false => ExecutionResult::SuccessNoStateChange(cmd.executed(updated_locations)),
        }
    }
}

impl fmt::Display for Minefield {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let delimiter = "-".repeat(self.fog.width());
        writeln!(f, "+{}+", delimiter)?;
        for row in self.fog.rows() {
            write!(f, "|")?;
            for element in row {
                write!(f, "{}", element)?;
            }
            writeln!(f, "|")?;
        }
        writeln!(f, "+{}+", delimiter)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn make_active_game() {
        let input = "eemeeeeeee
                          eeeeeeeefe
                          e012345678
                          eeeeeeeeee
                          eeFeeeeeME
                          eeeeeeeeee";
        let mf = Minefield::new_active_game(input);
        println!("{}", mf);
        let Minefield {
            fog, ground, state, ..
        } = mf;

        assert!(matches!(state, GameState::InProgress { .. }));
        assert_eq!((fog.width(), fog.height()), (10, 6));
        assert_eq!((ground.width(), ground.height()), (10, 6));

        let mut symbols = HashMap::new();
        symbols.insert('m', Location::new(2_usize, 0_usize));
        symbols.insert('f', Location::new(8_usize, 1_usize));
        symbols.insert('F', Location::new(2_usize, 4_usize));
        symbols.insert('M', Location::new(8_usize, 4_usize));
        symbols.insert('E', Location::new(9_usize, 4_usize));
        symbols.insert('e', Location::new(8_usize, 0_usize));
        symbols.extend((0..=8).map(|i| {
            (
                char::from_digit(i, 10).unwrap(),
                Location::new(i + 1, 2_usize),
            )
        }));

        let check = |symbol, state, ground_kind| {
            assert_eq!(
                fog[symbols[&symbol]], state,
                "Unexpected fog at {} with symbol {}.",
                symbols[&symbol], symbol
            );
            assert_eq!(
                ground[symbols[&symbol]], ground_kind,
                "Unexpected ground at {} with symbol {}.",
                symbols[&symbol], symbol
            );
        };
        check('m', State::Hidden, GroundKind::Mine);
        check('M', State::Exploded, GroundKind::Mine);
        check('f', State::Marked, GroundKind::Dirt);
        check('F', State::Marked, GroundKind::Mine);
        check('E', State::Revealed { adj_mines: 1 }, GroundKind::Dirt);
        check('e', State::Hidden, GroundKind::Dirt);
        for symbol in '0'..='8' {
            check(symbol, State::Revealed { adj_mines: 0 }, GroundKind::Dirt);
        }
    }
}
