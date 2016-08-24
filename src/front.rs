use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use rand::Rng;

use ast::*;

use eval::rem::Supervisor;

pub struct ModuleLoader {
    search_dirs: Vec<PathBuf>,
    loaded_modules: HashMap<Modpath, Module>,
    wanted_modules: HashSet<Modpath>,
}

#[derive(Debug)]
pub enum PathError {
    InvalidMainModuleName(OsString),
    AreYouSureThisIsARegularFile(OsString),
}

#[derive(Debug)]
pub enum CompileError {
    IoError(io::Error),
    PathError(PathError),
    ParseError(String),
    UnimplementedFeature(String),
}

pub type CompileResult<T> = Result<T, CompileError>;

impl From<io::Error> for CompileError {
    fn from(cause: io::Error) -> Self {
        CompileError::IoError(cause)
    }
}

impl From<PathError> for CompileError {
    fn from(cause: PathError) -> Self {
        CompileError::PathError(cause)
    }
}

fn get_stem(p: &Path) -> Result<&str, PathError> {
    let os_stem = try!(p.file_stem().ok_or({
        PathError::AreYouSureThisIsARegularFile(p.as_os_str().to_owned())
    }));
    
    // TODO: Check extension?

    os_stem.to_str().and_then(|str_stem| {
        use tokenizer::*;

        let mut tokens = Tokenizer::new(str_stem, 0);
        match tokens.next() {
            Some(Ok((0, Tok::NmFunc(_), _))) => match tokens.next() {
                None => Some(str_stem),
                _ => None,
            },
            _ => None,
        }
    }).ok_or(PathError::InvalidMainModuleName(os_stem.to_owned()))
}

#[test]
fn test_get_stem() {
    assert_eq!(get_stem(Path::new("/home/code/main.svr")).unwrap(), "main");
    assert!(get_stem(Path::new("not a valid name.svr")).is_err());
}

pub fn load(p: &Path) -> CompileResult<ModuleLoader> {
    let root_module = Modpath(vec![try!(get_stem(p)).to_string()]);

    let mut file = try!(File::open(p));
    let mut source = String::new();

    try!(file.read_to_string(&mut source));

    use tokenizer::Tokenizer;
    use parser::parse_Module;

    let module = match parse_Module(&source, Tokenizer::new(&source, 0)) {
        Ok(_) => unimplemented!(),
        Err(e) => return Err(CompileError::ParseError({
            format!("{:?}", e)
        }))
    };

    Err(CompileError::UnimplementedFeature(format!("everything")))
}

pub struct ModuleLoaderBuilder {
    search_dirs: Vec<PathBuf>,
    first_module_name: String,
}

impl ModuleLoaderBuilder {
    pub fn with_search_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.search_dirs = dirs;
        self
    }

    pub fn with_main_module(mut self, module_name: String) -> Self {
        self.first_module_name = module_name;
        self
    }

    pub fn build(self) -> ModuleLoader {
        let mut wanted_modules = HashSet::new();
        wanted_modules.insert(Modpath(vec![self.first_module_name]));

        ModuleLoader {
            search_dirs: self.search_dirs,
            wanted_modules: wanted_modules,
            loaded_modules: HashMap::new(),
        }
    }
}

impl ModuleLoader {
    pub fn new(dirs: Vec<PathBuf>, main: String) -> ModuleLoaderBuilder {
        ModuleLoaderBuilder {
            search_dirs: vec![PathBuf::from(".")],
            first_module_name: "story".into(),
        }
    }

    pub fn load_next(&mut self) -> CompileResult<()> {
        unimplemented!()
    }

    pub fn all_loaded(&self) -> bool {
        self.wanted_modules.is_empty()
    }

    pub fn compile(self) -> ! {
        unimplemented!()
    }
}
