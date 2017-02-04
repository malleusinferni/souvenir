use std::error::Error;
use std::io;
use std::path::Path;

use ast::{Program, Modpath, Module, ParseErr};

pub enum LoadErr {
    PathIsNotLoadable(String),
    ModpathIsNotUnicode(String),
    ModpathIsNotValid(String),
    Io(io::Error),
    Parse(String),
    Description(String),
}

impl Program {
    pub fn load(path: &Path) -> Result<Self, LoadErr> {
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
}

impl Modpath {
    fn from_path(path: &Path) -> Result<Self, LoadErr> {
        let display_path: String = path.to_string_lossy().into_owned();

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
