extern crate souvenir;
extern crate clap;

use souvenir::ast::Program;
use souvenir::driver::Try;

fn main() {
    use clap::{App, Arg};

    let matches = App::new("Souvenir demo interface")
        .arg(Arg::with_name("PATH")
             .index(1)
             .required(true)
             .help("Path to execute"))
        .arg(Arg::with_name("SCENE")
             .index(2)
             .required(true)
             .help("Scene to perform"))
        .get_matches();

    let filename = matches.value_of("PATH").unwrap();
    let scene = matches.value_of("SCENE").unwrap();

    run_demo(&filename, &scene)
        .unwrap();
}

use std::path::Path;

fn run_demo<P: AsRef<Path>>(path: P, scene: &str) -> Try<()> {
    let program = Program::load_from_path(path.as_ref())?.compile()?;

    let mut interpreter = program.init().unwrap();

    // FIXME: Run scene
    let _ = scene;

    loop {
        interpreter.dispatch();
    }
}
