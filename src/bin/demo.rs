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

    let actor = interpreter.spawn(scene, vec![]).unwrap();

    loop {
        interpreter.dispatch();

        if let Some(signal) = interpreter.read() {
            use souvenir::vm::OutSignal;

            match signal {
                OutSignal::Exit(id) => {
                    if id == actor {
                        println!("~ THE END ~");
                        break;
                    }
                },

                OutSignal::Hcf(_, err) => {
                    println!("Process died with an error: {:?}", err);
                    break;
                },

                OutSignal::Trace(_, value) => {
                    println!("{}", value);
                },

                OutSignal::Say(token) => {
                    println!("{}", token.content());
                    interpreter.write(token.reply().into());
                },

                _ => (),
            }
        }
    }

    Ok(())
}
