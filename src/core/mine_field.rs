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

pub struct Minefield {
    ground: Area<GroundKind>,
    fog: Area<State>,
}

impl Minefield {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            ground: Area::new(width, height),
            fog: Area::new(width, height),
        }
    }

    pub fn set_mines(&mut self, locations: impl Iterator<Item = Location>) {
        for l in locations {
            self.ground.get_mut(l).map(|g| *g = GroundKind::Mine);
        }
    }

    pub fn reveal(&mut self, location: Location) {
        let Minefield { ground, fog } = self;
        let s = match fog.get_mut(location) {
            Some(State::Revealed { .. } | State::Exploded) | None => return,
            Some(s) => s,
        };

        use super::location::Bounded;
        let pr = location.x == Bounded::Valid(39);
        if pr {
            print!("{:?}:  ", location);
        }
        let target = match ground.get(location) {
            Some(GroundKind::Dirt) => State::Revealed {
                adj_mines: location
                    .neighbours()
                    .inspect(|l| {
                        if pr {
                            print!("{:?} => ", l);
                        };
                    })
                    .filter_map(|l| ground.get(l).copied())
                    .inspect(|l| {
                        if pr {
                            print!("{:?};", l);
                        };
                    })
                    .filter(GroundKind::is_mine)
                    .count(),
            },
            Some(GroundKind::Mine) => {
                if pr {
                    print!("mine");
                }
                State::Exploded
            }
            None => return,
        };

        if pr {
            println!();
            println!();
        }
        *s = target;
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
