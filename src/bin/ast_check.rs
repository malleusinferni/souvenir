extern crate souvenir;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        println!("Needs a filename");
        return;
    }

    use souvenir::ast::Program;

    let program = Program::load_from_path(args[1].as_ref())
        .expect("Can't load program");

    match program.compile() {
        Ok(program) => println!("{:#?}", program),
        Err(e) => println!("{}", e),
    };
}
