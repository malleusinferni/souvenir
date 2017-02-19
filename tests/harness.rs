extern crate souvenir;

fn compile_single(modname: &str, source: &str) {
    use souvenir::ast::{Module, Modpath, Program};

    let modpath = Modpath(vec![modname.to_owned()]);

    let program = Program {
        modules: vec![
            (modpath, Module::parse(source).unwrap()),
        ],
    };

    program.compile().unwrap();
}

// See build.rs for source of generated code
include!(concat!(env!("OUT_DIR"), "/test_cases.rs"));
