#![allow(dead_code)]

mod core;
mod frontend;
mod generator;

fn main() {
    println!("Hello, world!");
    let mut mf = generator::simple_generate(20, 40, 20);
    for x in 0..40 {
        for y in 0..20 {
            mf.reveal((x, y).into())
        }
    }
    println!("{}", mf);
}
