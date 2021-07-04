use std::{fmt, iter, ops::Index};

use crate::core::Location;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Area<T> {
    area: Vec<T>,
    width: usize,
    height: usize,
}

impl<T> Area<T> {
    pub fn new(width: usize, height: usize) -> Self
    where
        T: Default + Clone,
    {
        Self {
            area: vec![Default::default(); width * height],
            width,
            height,
        }
    }

    pub fn get_mut(&mut self, l: Location) -> Option<&mut T> {
        let index = l.to_index(self.width)?;
        self.area.get_mut(index)
    }

    pub fn get(&self, l: Location) -> Option<&T> {
        let index = l.to_index(self.width)?;
        self.area.get(index)
    }

    pub fn rows(&self) -> impl Iterator<Item = &[T]> {
        self.area.chunks(self.width)
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.area.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.area.iter_mut()
    }
}

impl<T> Index<Location> for Area<T> {
    type Output = T;

    fn index(&self, l: Location) -> &Self::Output {
        let index = l.to_index(self.width).unwrap_or(0);
        &self.area[index]
    }
}

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

pub enum GameState {
    Lost,
    Won,
    InProgress,
}

pub struct Minefield {
    ground: Area<GroundKind>,
    pub fog: Area<State>,
}

impl Minefield {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            ground: Area::new(width, height),
            fog: Area::new(width, height),
        }
    }

    pub fn game_state(&self) -> GameState {
        let lost = self.fog.iter().any(State::is_exploded);
        let won = self
            .fog
            .iter()
            .zip(self.ground.iter())
            .all(|(&s, &g)| s.is_revealed() ^ (g.is_mine() && s == State::Marked));

        if lost {
            GameState::Lost
        } else if won {
            GameState::Won
        } else {
            GameState::InProgress
        }
    }

    pub fn set_mines(&mut self, locations: impl Iterator<Item = Location>) {
        for l in locations {
            self.ground.get_mut(l).map(|g| *g = GroundKind::Mine);
        }
    }

    pub fn width(&self) -> usize {
        self.fog.width
    }

    pub fn height(&self) -> usize {
        self.fog.height
    }

    pub fn mine_count(&self) -> usize {
        self.ground.iter().filter(|g| g.is_mine()).count()
    }

    pub fn mark_count(&self) -> usize {
        self.fog.iter().filter(|g| g.is_marked()).count()
    }

    pub fn reveal_all(&mut self) {
        let Minefield { ground, fog } = self;
        for (index, s) in fog.iter_mut().enumerate() {
            let location = Location::from_index(index, ground.width);
            Self::reveal_location(s, ground, location)
        }
    }

    fn reveal_location(state: &mut State, ground: &Area<GroundKind>, location: Location) {
        let target = match ground.get(location) {
            Some(GroundKind::Dirt) => State::Revealed {
                adj_mines: Self::count_mines_in_neighbourhood(ground, location),
            },
            Some(GroundKind::Mine) => State::Exploded,
            None => return,
        };
        *state = target;
    }

    fn count_mines_in_neighbourhood(ground: &Area<GroundKind>, location: Location) -> usize {
        location
            .neighbours()
            .filter_map(|l| ground.get(l).copied())
            .filter(GroundKind::is_mine)
            .count()
    }

    pub fn reveal(&mut self, location: Location) -> Option<State> {
        let Minefield { ground, fog } = self;
        let s = fog.get_mut(location).filter(|s| s.is_hidden())?;
        Self::reveal_location(s, ground, location);
        Some(*s)
    }

    pub fn unreveal(&mut self, location: Location) -> Option<State> {
        let s = self
            .fog
            .get_mut(location)
            .filter(|s| s.is_hidden() || s.is_revealed())?;
        *s = State::Hidden;
        Some(*s)
    }

    pub fn mark(&mut self, location: Location) -> Option<State> {
        let s = self.fog.get_mut(location).filter(|s| s.is_hidden())?;
        *s = State::Marked;
        Some(*s)
    }

    pub fn unmark(&mut self, location: Location) -> Option<State> {
        let s = self.fog.get_mut(location).filter(|s| s.is_marked())?;
        *s = State::Hidden;
        Some(*s)
    }

    pub fn toggle_mark(&mut self, location: Location) -> Option<State> {
        match self.fog.get_mut(location)? {
            State::Marked => self.unmark(location),
            State::Hidden => self.mark(location),
            _ => None,
        }
    }
}

impl fmt::Display for Minefield {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let delimiter: String = iter::repeat('-').take(self.fog.width).collect();
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
