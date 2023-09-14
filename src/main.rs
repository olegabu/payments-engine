mod account;
mod transaction;
mod engine;

use std::io;
use std::fs::File;
use clap::Parser;

use crate::engine::Engine;

#[derive(Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap()]
    filename: String,
}

fn main() {
    let args = Args::parse();

    let file = File::open(args.filename).expect("cannot open input file");

    let mut engine = Engine::new();

    engine.process(file, io::stdout());
}
