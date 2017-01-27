extern crate souvenir;

use std::io::Read;

use souvenir::ast::Module;

fn main() {
    let mut buf = String::new();

    match std::io::stdin().read_to_string(&mut buf) {
        Ok(_) => (),
        Err(_) => return,
    }

    let ast = Module::parse(&buf)
        .expect("Parse failed");

    println!("{:#?}", ast);
}
