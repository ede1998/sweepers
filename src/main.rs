use frontend::Term;

mod core;
mod frontend;
mod generator;
mod solver;

fn main() {
    let mut term = Term::new((None, None), None);
    term.go();
}
