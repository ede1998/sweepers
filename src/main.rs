#![allow(dead_code)]

use frontend::Term;

mod core;
mod frontend;
mod generator;

fn main() {
    let mut term = Term::new();
    term.init(None, None, 20);
    term.reset();
    term.go();
}
