extern crate lalrpop;

use std::env;
use std::io::{Read, Write};
use std::fs::{File, read_dir};
use std::path::PathBuf;

fn main() {
    lalrpop::process_root().unwrap();

    generate_compile_tests();
}

fn generate_compile_tests() {
    let mut outbuf = String::new();

    walk_dir("tests/valid/", |name, contents| {
        outbuf.push_str(&format!(r##"#[test]
fn {}() {{ compile_single({:?}, {:?}); }}
"##, name, name, contents));
    });

    walk_dir("tests/invalid/", |name, contents| {
        outbuf.push_str(&format!(r##"#[test]
#[should_panic]
fn {}() {{ compile_single({:?}, {:?}) }}
"##, name, name, contents));
    });

    let mut outfile = {
        let mut path = PathBuf::from(env::var("OUT_DIR").unwrap());
        path.push("test_cases.rs");
        File::create(&path).unwrap()
    };

    outfile.write_all(&outbuf.as_bytes()).unwrap();
}

fn walk_dir<F: FnMut(&str, &str)>(dir: &str, mut callback: F) {
    for entry in read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            let mut contents = String::new();
            File::open(entry.path()).unwrap()
                .read_to_string(&mut contents).unwrap();

            let mut name = PathBuf::from(entry.file_name());
            name.set_extension("");
            let name = format!("{}", name.display());

            callback(&name, &contents);
        }
    }
}
