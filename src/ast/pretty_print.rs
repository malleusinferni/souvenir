use std::fmt::*;

use ast;

impl Display for ast::Modpath {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0.join(":"))
    }
}

impl Display for ast::Label {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Label::Local { ref name } => write!(f, "'{}", name),
            &ast::Label::Anonymous => write!(f, ""),
        }
    }
}

impl Display for ast::Ident {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Ident::Var { ref name } => write!(f, "{}", name),
            &ast::Ident::Hole => write!(f, "_"),
            &ast::Ident::PidOfSelf => write!(f, "Self"),
        }
    }
}

impl Display for ast::Stmt {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Stmt::Empty => writeln!(f, ""),

            &ast::Stmt::Disarm { ref target } => {
                write!(f, "disarm {}", target)
            },

            &ast::Stmt::Let { ref name, ref value } => {
                write!(f, "let {} = {}", name, value)
            },

            &ast::Stmt::Listen { ref name, ref arms } => {
                writeln!(f, "listen {}", name)?;
                for arm in arms.iter() {
                    writeln!(f, "{}", arm)?;
                }
                writeln!(f, ";;")
            },

            _ => write!(f, "STATEMENT"),
        }
    }
}

impl Display for ast::TrapArm {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let &ast::TrapArm {
            ref pattern,
            ref origin,
            ref guard,
            ref body,
        } = self;

        writeln!(f, "| {} from {} when {}", pattern, origin, guard)?;
        for stmt in body.0.iter() {
            write!(f, "{}", format!("{}", stmt).indent_lines())?;
        }
        Ok(())
    }
}

impl Display for ast::Expr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Expr::Id(ref id) => write!(f, "{}", id),

            &ast::Expr::Lit(ref lit) => write!(f, "{}", lit),

            &ast::Expr::Str(_) => write!(f, "> I don't think so"),

            _ => write!(f, "EXPRESSION"),
        }
    }
}

impl Display for ast::Lit {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Lit::Atom(ref name) => write!(f, "#{}", name),

            &ast::Lit::Int(ref n) => write!(f, "{}", n),

            &ast::Lit::InvalidInt(ref s) => write!(f, "{}", s),
        }
    }
}

impl Display for ast::Pat {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &ast::Pat::Id(ref id) => write!(f, "{}", id),

            &ast::Pat::Lit(ref lit) => write!(f, "{}", lit),

            &ast::Pat::List(ref items) => write!(f, "[{}]", {
                items.iter()
                    .map(|i| format!("{}", i))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
        }
    }
}

impl Display for ast::FnName {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let &ast::FnName { ref name, ref in_module } = self;

        match in_module.as_ref() {
            Some(path) => write!(f, "{}:{}", path, name),
            None => write!(f, "{}", name),
        }
    }
}

impl Display for ast::QfdFnName {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let &ast::QfdFnName { ref name, ref in_module } = self;
        write!(f, "{}:{}", in_module, name)
    }
}

impl Display for ast::FnCall {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let &ast::FnCall(ref name, ref args) = self;

        let args = args.iter()
            .map(|expr| format!("{}", expr))
            .collect::<Vec<_>>();

        write!(f, "{}({})", name, args.join(", "))
    }
}

pub trait IndentLines {
    fn indent_lines(&self) -> String;
}

impl<'a> IndentLines for &'a str {
    fn indent_lines(&self) -> String {
        self.lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl IndentLines for String {
    fn indent_lines(&self) -> String {
        self.as_str().indent_lines()
    }
}

use driver::*;

impl Display for LoadErr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &LoadErr::Description(ref s) => write!(f, "{}", s),
            &LoadErr::Parse(ref s) => write!(f, "{}", s),

            &LoadErr::Io(ref err) => {
                use std::error::Error;
                write!(f, "{}", err.description())
            },

            &LoadErr::PathIsNotLoadable(ref path) => {
                write!(f, "Couldn't find modules in {}", path)
            },

            &LoadErr::ModpathIsNotUnicode(ref path) => {
                write!(f, "Unable to decode {:?}", path)
            },

            &LoadErr::ModpathIsNotValid(ref path) => {
                write!(f, "{:?} is not a valid module path", path)
            },
        }
    }
}

impl Display for CompileErr {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &CompileErr::Internal(ICE(ref ice)) => write!(f, "{}", ice),

            &CompileErr::Load(ref err) => write!(f, "{}", err),

            &CompileErr::BuildErrs(ref errs) => {
                for err in errs.iter() {
                    writeln!(f, "{}", err)?;
                }

                Ok(())
            },
        }
    }
}

impl Display for BuildErrWithCtx {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let &BuildErrWithCtx(ref cause, ref ctx) = self;

        match cause {
            &BuildErr::KnotWasOverqualified(ref name) => {
                writeln!(f, "Knot names shouldn't be qualified in their definitions:")?;
                write!(f, "{}", name.in_module.as_ref().unwrap())?;
            },

            &BuildErr::NoSuchModule(ref path) => {
                write!(f, "The module {} was not found.", path)?;
            },

            &BuildErr::NoSuchKnot(ref name) => {
                write!(f, "The knot {:?} was not found in the module {}.", &name.name, name.in_module)?;
            },

            &BuildErr::WrongNumberOfArgs { ref fncall, ref wanted, ref got } => {
                writeln!(f, "In the expression:\n{}", fncall)?;
                write!(f, "The function {} needs {} args, but was called with {}", &fncall.0.name, wanted, got)?;
            },

            &BuildErr::InvalidNumber(ref s) => {
                write!(f, "The number {} could not be parsed", s)?;
            },

            &BuildErr::IoInPrelude => {
                writeln!(f, "IO not allowed in module prelude")?;
            },

            &BuildErr::LabelInPrelude(ref _label) => {
                writeln!(f, "Traps not allowed in module prelude")?;
            },

            e => write!(f, "Can't describe this error yet: {:?}", e)?,
        };

        write!(f, "{}", ctx)
    }
}

impl Display for ErrCtx {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let stack = match self {
            &ErrCtx::Global(ref modpath, ref stack) => {
                writeln!(f, "  In module {}:", modpath)?;
                stack

            },

            &ErrCtx::Local(ref func_name, ref stack) => {
                writeln!(f, "  In the definition of knot {}", func_name)?;
                stack
            },

            &ErrCtx::NoContext => {
                return Ok(()) // Write nothing!
            },
        };

        if let Some(first) = stack.first() {
            writeln!(f, "{}", first.to_string().indent_lines())?;
        }

        if stack.len() > 1 {
            if let Some(last) = stack.last() {
                writeln!(f, "{}", last.to_string().indent_lines())?;
            }
        }

        Ok(())
    }
}
