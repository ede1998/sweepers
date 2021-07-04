#![allow(dead_code)]

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use frontend::Term;

mod core;
mod frontend;
mod generator;

fn main() {
    let mut term = Term::new();
    term.init(None, None, 20);
    term.reset();
    let start = Instant::now();
    eprintln!("start");
    while term.run() {
        eprintln!("running again.");
        sleep(Duration::from_millis(10));
    }
}
