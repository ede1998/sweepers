use std::ops::{Index, IndexMut};

use super::Location;

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

    pub fn with_area(width: usize, height: usize, area: Vec<T>) -> Self {
        Self {
            area,
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

    pub fn contains(&self, l: Location) -> bool {
        l.as_tuple()
            .map(|(x, y)| x < self.width && y < self.height)
            .unwrap_or(false)
    }

    /// Get a reference to the area's width.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get a reference to the area's height.
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn rows(&self) -> impl Iterator<Item = &[T]> {
        self.area.chunks(self.width)
    }

    pub fn loc_iter(&self) -> impl Iterator<Item = (Location, &T)> {
        Location::generate_all(self.width, self.height).zip(self.area.iter())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.area.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.area.iter_mut()
    }
}

impl<T> Default for Area<T> {
    fn default() -> Self {
        Self {
            area: vec![],
            height: 0,
            width: 0,
        }
    }
}

impl<T> IndexMut<Location> for Area<T> {
    fn index_mut(&mut self, l: Location) -> &mut Self::Output {
        let index = l.to_index(self.width).unwrap_or(0);
        &mut self.area[index]
    }
}

impl<T> Index<Location> for Area<T> {
    type Output = T;

    fn index(&self, l: Location) -> &Self::Output {
        let index = l.to_index(self.width).unwrap_or(0);
        &self.area[index]
    }
}
