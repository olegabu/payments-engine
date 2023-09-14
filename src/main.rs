mod account;
mod transaction;
mod engine;

use std::io;
use std::io::{Read, Write};
use std::fs::File;
use engine::Engine;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    #[clap()]
    filename: String,
}

fn run<R, W>(read: R, write: W)
where
    R: Read,
    W: Write,
{
    let mut engine = Engine::new();
    engine.input(read);
    if let Err(e) = engine.output(write) {
        eprintln!("Failed to serialize accounts: {}", e);
    }
}

fn main() {
    let args = Args::parse();

    let file = File::open(args.filename).expect("cannot open input file");

    run(file, io::stdout());
}
