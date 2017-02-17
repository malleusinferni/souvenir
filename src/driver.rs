use std::error::Error;
use std::io;
use std::path::Path;

use ast::{self, Program, Modpath, Module, ParseErr};

use vm;

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
    BuildErrs(Vec<BuildErrWithCtx>),
}

#[derive(Clone, Debug)]
pub enum ErrCtx {
    Global(Modpath, Vec<ast::Stmt>),
    Local(ast::QfdSceneName, Vec<ast::Stmt>),
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
    NoSuchScene(ast::QfdSceneName),
    NoSuchLabel(ast::Label),
    NoSuchVar(String),
    InvalidNumber(String),
    InvalidAssignToSelf(ast::Stmt),
    InvalidAssignToHole(ast::Stmt),
    SceneWasRedefined(ast::QfdSceneName),
    SceneWasOverqualified(ast::SceneName),
    IoInPrelude,
    SelfInPrelude,
    LabelInPrelude(ast::Label),
    LabelRedefined(ast::Label),
    WrongNumberOfArgs {
        call: ast::Call,
        wanted: usize,
        got: usize,
    },
    MultipleErrors(Vec<BuildErrWithCtx>),
}

#[derive(Clone, Debug)]
pub struct BuildErrWithCtx(pub BuildErr, pub ErrCtx);

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

    pub fn compile(self) -> Result<vm::Program, CompileErr> {
        self.check_names()?;
        self.check_prelude_restrictions()?;
        let ir = self.desugar()?.translate()?;
        let bytecode = ir.translate()?;
        Ok(bytecode)
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
            ErrCtx::Global(modpath, mut stack) => {
                match stack.pop() {
                    Some(_) => ErrCtx::Global(modpath, stack),
                    None => ErrCtx::NoContext,
                }
            },

            ErrCtx::Local(scene, mut stack) => {
                match stack.pop() {
                    Some(_) => ErrCtx::Local(scene, stack),
                    None => ErrCtx::Global(scene.in_module, vec![]),
                }
            },

            ErrCtx::NoContext => ice!("Spurious exit from error context"),
        };

        Ok(())
    }

    pub fn modpath(&self) -> Try<Modpath> {
        match self {
            &ErrCtx::Global(ref path, _) => Ok(path.clone()),
            &ErrCtx::Local(ast::QfdSceneName { ref in_module, .. }, _) => {
                Ok(in_module.clone())
            },
            _ => ice!("No module path in error context"),
        }
    }

    pub fn begin_module(&mut self, path: &Modpath) {
        *self = ErrCtx::Global(path.clone(), vec![]);
    }

    pub fn begin_scene(&mut self, name: &str) -> Try<()> {
        let qualified = ast::QfdSceneName {
            name: name.to_owned(),
            in_module: self.modpath()?,
        };

        *self = ErrCtx::Local(qualified, vec![]);

        Ok(())
    }

    pub fn push_stmt(&mut self, stmt: &ast::Stmt) -> Try<()> {
        match self {
            &mut ErrCtx::Local(_, ref mut stack) => {
                stack.push(stmt.clone());
            },

            &mut ErrCtx::Global(_, ref mut stack) => {
                stack.push(stmt.clone());
            },

            _ => ice!("Statement outside of error context"),
        }

        Ok(())
    }
}

impl BuildErr {
    pub fn with_ctx(self, ctx: &ErrCtx) -> BuildErrWithCtx {
        BuildErrWithCtx(self, ctx.clone())
    }
}

impl From<io::Error> for LoadErr {
    fn from(err: io::Error) -> Self {
        LoadErr::Io(err)
    }
}

impl<'a> From<ParseErr<'a>> for LoadErr {
    fn from(err: ParseErr<'a>) -> Self {
        LoadErr::Parse(match err.cause() {
            Some(err) => format!("{}: {}", err.description(), err),
            None => format!("{:?}", err)
        })
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

impl From<Vec<BuildErrWithCtx>> for CompileErr {
    fn from(errs: Vec<BuildErrWithCtx>) -> Self {
        CompileErr::BuildErrs(errs)
    }
}
