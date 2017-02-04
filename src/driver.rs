use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

use ast::{self, Program, Modpath, Module, ParseErr};

macro_rules! ice {
    ( $( $arg:tt )* ) => {
        return Err(::driver::ICE(format!($($arg)*)).into())
    }
}

pub type Try<T> = Result<T, CompileErr>;

#[derive(Debug)]
pub enum CompileErr {
    Internal(ICE),
    Load(LoadErr),
    BuildErrs(Vec<(BuildErr, ErrCtx)>),
}

#[derive(Clone, Debug)]
pub enum ErrCtx {
    Prelude(Modpath, Vec<ast::Stmt>),
    KnotDef(Modpath, ast::FnName),
    Local(Modpath, ast::FnName, Vec<ast::Stmt>),
    NoContext,
}

#[derive(Clone, Debug)]
pub struct ICE(pub String);

#[derive(Debug)]
pub enum LoadErr {
    PathIsNotLoadable(String),
    ModpathIsNotUnicode(String),
    ModpathIsNotValid(String),
    Io(io::Error),
    Parse(String),
    Description(String),
}

#[derive(Clone, Debug)]
pub enum BuildErr {
    NoSuchModule(Modpath),
    NoSuchKnot(ast::QfdFnName),
    NoSuchLabel(ast::Label),
    NoSuchVar(String),
    InvalidNumber(String),
    InvalidAssignToSelf(ast::Stmt),
    InvalidAssignToHole(ast::Stmt),
    KnotWasRedefined(ast::QfdFnName),
    KnotWasOverqualified,
    IoInPrelude,
    LabelInPrelude(ast::Label),
    LabelRedefined(ast::Label),
    WrongNumberOfArgs {
        fncall: ast::FnCall,
        wanted: usize,
        got: usize,
    },
    MultipleErrors(Vec<(BuildErr, ErrCtx)>),
}

impl Program {
    pub fn load_from_path(path: &Path) -> Result<Self, LoadErr> {
        let mut dirs = Vec::with_capacity(16);

        let mut files = Vec::with_capacity(16);

        let root_dir = if path.is_dir() {
            let dir = path.to_owned();
            dirs.push(dir.clone());
            dir
        } else if path.is_file() {
            // Only load a single file
            let file = path.to_owned();
            files.push(file.clone());

            file.parent()
                .ok_or("Can't find parent directory")?
                .to_owned()
        } else {
            return Err(LoadErr::PathIsNotLoadable({
                path.to_string_lossy().into_owned()
            }));
        };

        while let Some(dir) = dirs.pop() {
            for entry in dir.read_dir()? {
                let entry = entry?;
                let path = entry.path();
                let file_type = entry.file_type()?;

                if file_type.is_file() {
                    files.push(path);
                } else if file_type.is_dir() {
                    dirs.push(path);
                } else {
                    // Ignore and continue, I guess
                }
            }
        }

        let mut modules = Vec::with_capacity(files.len());

        let mut source = String::new();

        for path in files.into_iter() {
            let subpath = path.strip_prefix(&root_dir)
                .map_err(|e| e.description().to_string())?;

            let modpath = Modpath::from_path(subpath)?;

            use std::fs::File;
            use std::io::Read;

            let mut file = File::open(&path)?;
            file.read_to_string(&mut source)?;

            let ast = Module::parse(&source)?;

            modules.push((modpath, ast));

            source.clear();
        }

        Ok(Program {
            modules: modules
        })
    }

    pub fn compile(self) -> Result<Self, CompileErr> {
        self.check_names()?;
        Ok(self)
    }
}

impl Modpath {
    fn from_path(path: &Path) -> Result<Self, LoadErr> {
        let display_path: String = path.to_string_lossy().into_owned();

        let path = path.with_extension("");

        let mut elements = Vec::new();
        for component in path.components() {
            let element = component.as_os_str().to_str()
                .ok_or(LoadErr::ModpathIsNotUnicode(display_path.clone()))?
                .to_owned();

            if element.chars().all(|ch| ch.is_lowercase() || ch == '_') {
                elements.push(element);
            } else {
                return Err(LoadErr::ModpathIsNotValid(display_path));
            }
        }

        Ok(Modpath(elements))
    }
}

impl ErrCtx {
    pub fn pop(&mut self) -> Try<()> {
        *self = match self.clone() {
            ErrCtx::Prelude(modpath, mut stack) => {
                match stack.pop() {
                    Some(_) => ErrCtx::Prelude(modpath, stack),
                    None => ErrCtx::NoContext,
                }
            },

            ErrCtx::KnotDef(_, _) => ErrCtx::NoContext,

            ErrCtx::Local(modpath, knot_name, mut stack) => {
                match stack.pop() {
                    Some(_) => ErrCtx::Local(modpath, knot_name, stack),
                    None => ErrCtx::NoContext,
                }
            },

            ErrCtx::NoContext => ice!("Spurious exit from error context"),
        };

        Ok(())
    }
}

impl From<io::Error> for LoadErr {
    fn from(err: io::Error) -> Self {
        LoadErr::Io(err)
    }
}

impl<'a> From<ParseErr<'a>> for LoadErr {
    fn from(err: ParseErr<'a>) -> Self {
        // FIXME: Implement Display for TokErr and use .description()
        LoadErr::Parse(format!("{:?}", err))
    }
}

impl<'a> From<&'a str> for LoadErr {
    fn from(err: &'a str) -> Self {
        LoadErr::Description(err.to_string())
    }
}

impl From<String> for LoadErr {
    fn from(err: String) -> Self {
        LoadErr::Description(err)
    }
}

impl fmt::Display for LoadErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LoadErr::Description(ref s) => writeln!(f, "{}", s),
            &LoadErr::Parse(ref s) => writeln!(f, "{}", s),

            &LoadErr::Io(ref err) => {
                writeln!(f, "{}", err.description())
            },

            &LoadErr::PathIsNotLoadable(ref path) => {
                writeln!(f, "Couldn't find modules in {}", path)
            },

            &LoadErr::ModpathIsNotUnicode(ref path) => {
                writeln!(f, "Unable to decode {:?}", path)
            },

            &LoadErr::ModpathIsNotValid(ref path) => {
                writeln!(f, "{:?} is not a valid module path", path)
            },
        }
    }
}

impl From<LoadErr> for CompileErr {
    fn from(err: LoadErr) -> Self {
        CompileErr::Load(err)
    }
}

impl From<ICE> for CompileErr {
    fn from(ice: ICE) -> Self {
        CompileErr::Internal(ice)
    }
}

impl From<Vec<(BuildErr, ErrCtx)>> for CompileErr {
    fn from(errs: Vec<(BuildErr, ErrCtx)>) -> Self {
        CompileErr::BuildErrs(errs)
    }
}
