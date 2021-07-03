use core::*;

use crate::core::{Location, Minefield};
use rand::seq::index::sample as rand_sample;

pub fn simple_generate(mine_count: usize, width: usize, height: usize) -> Minefield {
    let result = rand_sample(&mut rand::thread_rng(), width * height, mine_count);
    let mines = result.into_iter().map(|i| Location::from_index(i, width));
    let mut mf = Minefield::new(width, height);
    mf.set_mines(mines);
    mf
}
