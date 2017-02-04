use std::path::PathBuf;

fn main() {
    let mut path = PathBuf::from("/usr/bin/binhex.pl");
    path.set_extension("");
    println!("{}", path.display());
}
