use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use rand::Rng;

use ast::*;

use eval::rem::Supervisor;

const EXT: &'static str = "svr";

pub struct ModuleLoader {
    search_dirs: Vec<PathBuf>,
    loaded_modules: HashMap<Modpath, Module>,
    wanted_modules: HashMap<Modpath, WantedBy>,
    name_of_main: Option<Label>,
}

#[derive(Debug)]
pub enum WantedBy {
    User,
    OtherModule(Modpath),
}

#[derive(Debug)]
pub enum PathError {
    InvalidMainModuleName(OsString),
    AreYouSureThisIsARegularFile(OsString),
    SearchedAllThesePlaces(Vec<PathBuf>, Modpath, WantedBy),
}

#[derive(Debug)]
pub enum CompileError {
    IoError(io::Error),
    PathError(PathError),
    ParseError(String),
    MissingModules(Vec<Modpath>),
    NoModulesLoaded,
    MainIsMissing,
    UnimplementedFeature(String),
    CantQualifyLabel(Label),
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

// impl From<???> for CompileError { ... }
macro_rules! try_parse {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            Err(e) => return Err(CompileError::ParseError(format!("{:?}", e))),
        }
    }
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
        let mut wanted_modules = HashMap::new();
        let root = Modpath(vec![self.first_module_name]);
        wanted_modules.insert(root, WantedBy::User);

        ModuleLoader {
            search_dirs: self.search_dirs,
            wanted_modules: wanted_modules,
            loaded_modules: HashMap::new(),
            name_of_main: None,
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
        use self::hash_queue::*;

        let (modpath, wanted_by) = match self.wanted_modules.pop() {
            Some(m) => m,
            None => return Ok(()),
        };

        let path = try!(self.guess_filename(&modpath, wanted_by));

        let mut file = try!(File::open(&path));
        let mut source = String::new();
        try!(file.read_to_string(&mut source));

        let module = try_parse!({
            use tokenizer::Tokenizer;
            use parser::parse_Module;

            parse_Module(&source, Tokenizer::new(&source, 0))
        });

        for wanted_module in module.wanted_modules() {
            if self.loaded_modules.contains_key(&wanted_module) { continue; }

            self.wanted_modules.entry(wanted_module).or_insert({
                WantedBy::OtherModule(modpath.clone())
            });
        }

        if self.name_of_main.is_none() {
            if let Some(knot) = module.knots.iter().next() {
                self.name_of_main = Some({
                    try!(knot.name.qualify(modpath.clone()))
                });
            } else {
                return Err(CompileError::MainIsMissing);
            }
        }

        self.loaded_modules.insert(modpath, module);

        Ok(())
    }

    fn guess_filename(&self, modpath: &Modpath, wanted_by: WantedBy) -> CompileResult<PathBuf> {
        let &Modpath(ref segments) = modpath;
        let mut paths_tried = vec![];

        for mut buf in self.search_dirs.iter().cloned() {
            for seg in segments.iter() {
                buf.push(seg);
            }

            buf.set_extension(EXT);

            let path = try!(buf.canonicalize());

            if path.is_file() {
                return Ok(path)
            } else {
                paths_tried.push(buf);
            }
        }

        let modpath = modpath.clone();
        Err(PathError::SearchedAllThesePlaces(paths_tried, modpath, wanted_by).into())
    }

    pub fn all_loaded(&self) -> bool {
        self.wanted_modules.is_empty()
    }

    pub fn compile<R: Rng>(self, rng: R) -> CompileResult<Supervisor<R>> {
        if !self.wanted_modules.is_empty() {
            return Err(CompileError::MissingModules({
                self.wanted_modules.into_iter().map(|(m, _)| m).collect()
            }));
        }

        if self.loaded_modules.is_empty() {
            return Err(CompileError::NoModulesLoaded);
        }

        use mid::*;

        let mut tree_walker = TreeWalker::new(self.loaded_modules);

        unimplemented!()
    }
}

impl Module {
    fn wanted_modules(&self) -> HashSet<Modpath> {
        let mut wanted = HashSet::new();
        for knot in self.knots.iter() {
            for stmt in knot.body.iter() {
                stmt.tally(&mut wanted);
            }
        }
        wanted
    }
}

impl Stmt {
    fn tally(&self, wanted: &mut HashSet<Modpath>) {
        match self {
            &Stmt::Empty => (),

            &Stmt::Disarm(_) => (),

            &Stmt::Let(_, ref expr) => expr.tally(wanted),

            &Stmt::LetSpawn(_, ref label, ref exprs) => {
                label.tally(wanted);
                for expr in exprs.iter() {
                    expr.tally(wanted);
                }
            },

            &Stmt::Listen(ref traps) => for trap in traps.iter() {
                trap.guard.tally(wanted);
                for stmt in trap.body.iter() {
                    stmt.tally(wanted);
                }
            },

            &Stmt::SendMsg(_, ref expr) => expr.tally(wanted),

            &Stmt::TailCall(ref label, _) => label.tally(wanted),

            &Stmt::Trace(ref expr) => expr.tally(wanted),

            &Stmt::Trap(_, ref traps) => for trap in traps.iter() {
                trap.guard.tally(wanted);
                for stmt in trap.body.iter() {
                    stmt.tally(wanted);
                }
            },

            &Stmt::Wait(ref expr) => expr.tally(wanted),

            &Stmt::Weave(_, ref choices) => for choice in choices.iter() {
                choice.guard.tally(wanted);
                choice.title.tally(wanted);
                for stmt in choice.body.iter() {
                    stmt.tally(wanted);
                }
            },
        }
    }
}

impl Expr {
    fn tally(&self, wanted: &mut HashSet<Modpath>) {
        match self {
            &Expr::Count(ref label) => label.tally(wanted),

            &Expr::Not(ref expr) => expr.tally(wanted),

            &Expr::List(ref exprs) => for expr in exprs.iter() {
                expr.tally(wanted);
            },

            &Expr::Binop(ref lhs, _, ref rhs) => {
                lhs.tally(wanted);
                rhs.tally(wanted);
            },

            _ => (),
        }
    }
}

impl Label {
    fn tally(&self, wanted: &mut HashSet<Modpath>) {
        match self {
            &Label::Qualified(ref modpath, _) => {
                wanted.insert(modpath.clone());
            },

            _ => (),
        }
    }

    fn qualify(&self, modpath: Modpath) -> CompileResult<Self> {
        match self {
            &Label::Local(ref n) => Ok(Label::Qualified(modpath, n.clone())),
            _ => Err(CompileError::CantQualifyLabel(self.clone())),
        }
    }
}

mod hash_queue {
    use std::hash::Hash;
    use std::collections::HashMap;

    pub trait HashQueue<K, T> {
        fn pop(&mut self) -> Option<(K, T)>;
    }

    impl<K, T> HashQueue<K, T> for HashMap<K, T> where K: Hash + Eq + Clone {
        fn pop(&mut self) -> Option<(K, T)> {
            match self.keys().cloned().next() {
                None => None,
                Some(key) => self.remove(&key).map(|v| (key, v)),
            }
        }
    }
}
