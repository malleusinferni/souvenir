extern crate souvenir;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

fn main() {
    use souvenir::eval::*;

    let filename = match std::env::args_os().nth(1) {
        Some(filename) => filename,
        None => {
            println!("No filename given");
            return;
        },
    };

    let path = Path::new(&filename);

    let stem = match path.file_stem() {
        Some(stem) => stem.to_str().expect("Invalid UTF-8"),
        None => {
            println!("Are you sure this is a regular file?");
            return;
        },
    };

    let mut file = File::open(&filename)
        .expect("Can't open file");

    let mut source = String::new();
    file.read_to_string(&mut source)
        .expect("Can't read file contents");

    let mut supervisor = Evaluator::new(100.0);
    supervisor.compile(stem, &source).expect("Compile error");

    let start = souvenir::ast::Label::Explicit("start".to_owned());
    supervisor.spawn(start, vec![]).expect("Can't start process");

    loop {
        let state = supervisor.dispatch(0.25);
        supervisor.with_stdout(|s| {
            let millis = s.len() * 25;
            println!("{}\n", s);
            std::thread::sleep(Duration::from_millis(millis as u64));
        });

        match state {
            RunState::Running | RunState::Sleeping(_) => {
                std::thread::sleep(Duration::from_millis(250));
            },

            RunState::Idling => {
                println!("Good end (?)");
                return;
            },

            RunState::SelfTerminated => {
                println!("Bad end (?)");
                return;
            },

            RunState::OnFire(e) => {
                let string: String = e.into();
                println!("{}", string);
                return;
            },

            RunState::WaitingForInput(options) => {
                for (i, (text, _)) in options.into_iter().enumerate() {
                    println!("{}: {}", i + 1, text);
                }

                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)
                    .expect("Oh no");

                supervisor.choose(match buf.trim().parse::<usize>() {
                    Ok(n) if n > 0 => n - 1,
                    Ok(_) => panic!("Bad choice"),
                    Err(e) => panic!("{}: {}", buf, e),
                });
            },
        }
    }
}
