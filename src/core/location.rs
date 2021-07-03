use std::{
    array::IntoIter,
    ops::{Add, Mul, Sub},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bounded {
    Invalid,
    Valid(usize),
}

impl Bounded {
    fn op<F: Fn(usize, usize) -> Option<usize>>(self, other: Self, operation: F) -> Self {
        use Bounded::*;
        match (self, other) {
            (Valid(lhs), Valid(rhs)) => operation(lhs, rhs).map_or(Invalid, Valid),
            _ => Invalid,
        }
    }
}

impl From<usize> for Bounded {
    fn from(f: usize) -> Self {
        Self::Valid(f)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    pub x: Bounded,
    pub y: Bounded,
}

const ONE: Bounded = Bounded::Valid(1);

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
        IntoIter::new([
            self.up().left(),
            self.up(),
            self.up().right(),
            self.left(),
            self.right(),
            self.down().left(),
            self.down(),
            self.down().right(),
        ])
    }

    fn map_x<F: FnOnce(Bounded) -> Bounded>(mut self, f: F) -> Self {
        self.x = f(self.x);
        self
    }

    fn map_y<F: FnOnce(Bounded) -> Bounded>(mut self, f: F) -> Self {
        self.y = f(self.y);
        self
    }

    fn left(self) -> Self {
        self.map_x(|x| x - ONE)
    }

    fn right(self) -> Self {
        self.map_x(|x| x + ONE)
    }

    fn up(self) -> Self {
        self.map_y(|y| y - ONE)
    }

    fn down(self) -> Self {
        self.map_y(|y| y + ONE)
    }
}

impl From<(usize, usize)> for Location {
    fn from((x, y): (usize, usize)) -> Self {
        Location::new(x, y)
    }
}
