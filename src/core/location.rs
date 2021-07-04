use std::{
    array::IntoIter,
    fmt,
    ops::{Add, Mul, Sub},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bounded {
    Invalid,
    Valid(usize),
}

impl fmt::Display for Bounded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Bounded::Invalid => write!(f, "NaN"),
            Bounded::Valid(s) => write!(f, "{}", s),
        }
    }
}

impl Default for Bounded {
    fn default() -> Self {
        Self::Invalid
    }
}

impl Bounded {
    fn op<F: Fn(usize, usize) -> Option<usize>>(self, other: Self, operation: F) -> Self {
        use Bounded::*;
        match (self, other) {
            (Valid(lhs), Valid(rhs)) => operation(lhs, rhs).map_or(Invalid, Valid),
            _ => Invalid,
        }
    }

    /// Returns `true` if the bounded is [`Invalid`].
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid)
    }
}

impl From<usize> for Bounded {
    fn from(f: usize) -> Self {
        Self::Valid(f)
    }
}

impl From<u16> for Bounded {
    fn from(f: u16) -> Self {
        Self::Valid(f.into())
    }
}

impl From<Bounded> for Option<usize> {
    fn from(f: Bounded) -> Self {
        match f {
            Bounded::Invalid => None,
            Bounded::Valid(s) => Some(s),
        }
    }
}

impl Sub for Bounded {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.op(rhs, usize::checked_sub)
    }
}

impl Add for Bounded {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.op(rhs, usize::checked_add)
    }
}

impl Mul for Bounded {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.op(rhs, usize::checked_mul)
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    pub x: Bounded,
    pub y: Bounded,
}

impl Location {
    pub fn new<I, J>(x: I, y: J) -> Self
    where
        I: Into<Bounded>,
        J: Into<Bounded>,
    {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn to_index(&self, width: usize) -> Option<usize> {
        match self.x {
            Bounded::Valid(x) if x < width => (self.y * width.into() + self.x).into(),
            _ => None,
        }
    }

    pub fn from_index(index: usize, width: usize) -> Self {
        Self {
            x: (index % width).into(),
            y: (index / width).into(),
        }
    }

    pub fn neighbours(&self) -> impl Iterator<Item = Location> {
        use Direction::*;
        IntoIter::new([
            self.mv(Up).mv(Left),
            self.mv(Up),
            self.mv(Up).mv(Right),
            self.mv(Left),
            self.mv(Right),
            self.mv(Down).mv(Left),
            self.mv(Down),
            self.mv(Down).mv(Right),
        ])
    }

    pub fn map_x<F: FnOnce(Bounded) -> Bounded>(mut self, f: F) -> Self {
        self.x = f(self.x);
        self
    }

    pub fn map_y<F: FnOnce(Bounded) -> Bounded>(mut self, f: F) -> Self {
        self.y = f(self.y);
        self
    }

    pub fn mv(self, d: Direction) -> Self {
        let one = Bounded::Valid(1);
        match d {
            Direction::Left => self.map_x(|x| x - one),
            Direction::Right => self.map_x(|x| x + one),
            Direction::Up => self.map_y(|y| y - one),
            Direction::Down => self.map_y(|y| y + one),
        }
    }

    pub fn try_mv(self, d: Direction) -> Self {
        let original = self;
        let new = self.mv(d);
        match new.x.is_invalid() || new.y.is_invalid() {
            true => original,
            false => new,
        }
    }

    pub fn as_tuple(self) -> Option<(usize, usize)> {
        let x: Option<_> = self.x.into();
        let y: Option<_> = self.y.into();
        x.zip(y)
    }
}

impl From<(usize, usize)> for Location {
    fn from((x, y): (usize, usize)) -> Self {
        Location::new(x, y)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
