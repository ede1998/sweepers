use std::{
    array::IntoIter,
    convert::TryInto,
    fmt,
    ops::{Add, AddAssign, Mul, Sub, SubAssign},
};

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

impl From<u32> for Bounded {
    fn from(f: u32) -> Self {
        Self::Valid(f.try_into().unwrap())
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

impl AddAssign for Bounded {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Bounded {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Bounded {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.op(rhs, usize::checked_mul)
    }
}

#[derive(Default, Hash, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    pub x: Bounded,
    pub y: Bounded,
}

impl Location {
    pub const Invalid: Location = Location {
        x: Bounded::Invalid,
        y: Bounded::Invalid,
    };

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

    pub fn generate_all(width: usize, height: usize) -> impl Iterator<Item = Self> {
        (0..width * height).map(move |i| Self::from_index(i, width))
    }

    pub fn to_index(self, width: usize) -> Option<usize> {
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

    pub fn x_plus<I: Into<Bounded>>(mut self, num: I) -> Self {
        let num = num.into();
        self.x += num;
        self
    }

    pub fn x_minus<I: Into<Bounded>>(mut self, num: I) -> Self {
        let num = num.into();
        self.x -= num;
        self
    }

    pub fn y_plus<I: Into<Bounded>>(mut self, num: I) -> Self {
        let num = num.into();
        self.y += num;
        self
    }

    pub fn y_minus<I: Into<Bounded>>(mut self, num: I) -> Self {
        let num = num.into();
        self.y -= num;
        self
    }

    pub fn mv(self, d: Direction) -> Self {
        match d {
            Direction::Left => self.x_minus(1u16),
            Direction::Right => self.x_plus(1u16),
            Direction::Up => self.y_minus(1u16),
            Direction::Down => self.y_plus(1u16),
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
