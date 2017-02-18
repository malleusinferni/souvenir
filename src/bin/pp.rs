extern crate souvenir;
extern crate clap;

enum Cmd {
    DumpAst,
    DumpRem,
}

fn main() {
    use clap::{App, Arg};

    let matches = App::new("Souvenir pretty-printer")
        .arg(Arg::with_name("ast")
             .long("ast")
             .help("Print the AST instead of the compiled code"))
        .arg(Arg::with_name("PATH")
             .index(1)
             .required(true)
             .help("Path to Souvenir source file or directory"))
        .get_matches();

    let filename = matches.value_of("PATH").unwrap();

    let cmd = match matches.occurrences_of("ast") {
        0 => Cmd::DumpRem,
        _ => Cmd::DumpAst,
    };

    if let Err(err) = pp(filename, cmd) {
        println!("Error:");
        println!("{}", err);
    }
}

use souvenir::ast::Program;
use souvenir::driver::Try;

fn pp(path: &str, cmd: Cmd) -> Try<()> {
    let program = Program::load_from_path(path.as_ref())?;

    match cmd {
        Cmd::DumpAst => {
            println!("{:#?}", program);
        },

        Cmd::DumpRem => {
            println!("{}", program.compile()?);
        },
    };

    Ok(())
}
