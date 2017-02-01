pub mod from_ast;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    pub preludes: Vec<Scope>,
    pub knots: Vec<KnotDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KnotDef {
    pub args_wanted: u32,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: Pat,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Arm {
        name: Label,
        body: Scope,
    },

    Disarm {
        name: Label,
    },

    Discard {
        value: Expr,
    },

    If {
        test: Expr,
        success: Scope,
        failure: Scope,
    },

    Let {
        value: Expr,
        dest: Reg,
    },

    Match {
        value: Expr,
        arms: Vec<MatchArm>,
        failure: Scope,
    },

    Recur {
        target: FnCall,
    },

    Return {
        result: bool,
    },

    SendMsg {
        message: Expr,
        target: Expr,
    },

    Trace {
        value: Expr,
    },

    Wait {
        value: Expr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Atom(Atom),
    Id(Reg),
    Int(i32),
    List(Vec<Expr>),
    Spawn(FnCall),
    Strcat(Vec<Expr>),
    Strlit(String),
    PidOfSelf,
    PidZero,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Hole,
    Assign(Reg),
    EqualTo(Expr),
    List(Vec<Pat>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    MenuItem,
    MenuEnd,
    LastResort,
    PrintLine,
    PrintFinished,
    User(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnCall(pub FnId, pub Vec<Expr>);

#[derive(Clone, Debug, PartialEq)]
pub struct Label(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Reg(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FnId(pub u32);

use std::fmt::{Display, Error, Formatter, Write};

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "#{}", match self {
            &Atom::User(ref s) => s.as_ref(),
            &Atom::MenuItem => "[menu item]",
            &Atom::MenuEnd => "[show menu]",
            &Atom::LastResort => "[last resort]",
            &Atom::PrintLine => "[print line]",
            &Atom::PrintFinished => "[print finished]",
        })
    }
}

impl Display for Pat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Pat::Hole => write!(f, "_"),
            &Pat::Assign(ref r) => write!(f, "{}", r),
            &Pat::EqualTo(ref e) => write!(f, "{}", e),
            &Pat::List(ref items) => {
                write!(f, "[")?; items.pp_slice(f)?; write!(f, "]")
            },
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Expr::Atom(ref a) => write!(f, "{}", a),
            &Expr::Id(ref p) => write!(f, "{}", p),
            &Expr::Int(n) => write!(f, "{}", n),
            &Expr::List(ref items) => {
                write!(f, "[")?; items.pp_slice(f)?; write!(f, "]")
            },
            &Expr::Spawn(ref fncall) => write!(f, "spawn {}", fncall),
            &Expr::Strcat(ref items) => write!(f, "{:?}", strcat(items)),
            &Expr::Strlit(ref s) => write!(f, "> {}", s),
            &Expr::PidOfSelf => write!(f, "Self"),
            &Expr::PidZero => write!(f, "%stdio"),
        }
    }
}

impl Display for FnCall {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let FnId(n) = self.0;
        write!(f, ":{}: (", n)?;
        self.1.pp_slice(f)?;
        write!(f, ")")
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "%{}", self.0)
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "'{:0x}", self.0)
    }
}

fn strcat(items: &[Expr]) -> String {
    let mut out = String::new();
    for item in items { write!(out, "{}", item).unwrap(); }
    out
}

trait PpSlice {
    fn pp_slice(&self, &mut Formatter) -> Result<(), Error>;
}

impl<T: Display> PpSlice for [T] {
    fn pp_slice(&self, f: &mut Formatter) -> Result<(), Error> {
        let mut first = true;
        for item in self {
            if first {
                first = false;
                write!(f, "{}", item)?;
            } else {
                write!(f, ", {}", item)?;
            }
        }

        Ok(())
    }
}

fn indent(f: &mut Formatter, depth: u32) -> Result<(), Error> {
    for _ in 0 .. depth {
        write!(f, "  ")?;
    }

    Ok(())
}

impl Scope {
    pub fn pp(&self, f: &mut Formatter, depth: u32) -> Result<(), Error> {
        for stmt in &self.body {
            stmt.pp(f, depth + 1)?;
        }

        Ok(())
    }
}

impl Stmt {
    pub fn pp(&self, f: &mut Formatter, depth: u32) -> Result<(), Error> {
        indent(f, depth)?;

        match self {
            &Stmt::Arm { ref name, ref body } => {
                writeln!(f, "trap {} = lambda %msg, %sender:", name)?;
                body.pp(f, depth)?;
                indent(f, depth)?; writeln!(f, ";;")
            },

            &Stmt::Disarm { ref name } => {
                writeln!(f, "disarm {}", name)
            },

            &Stmt::Discard { ref value } => {
                writeln!(f, "let _ = {}", value)
            },

            &Stmt::If { ref test, ref success, ref failure } => {
                writeln!(f, "if {}:", test)?;
                success.pp(f, depth)?;
                indent(f, depth)?; writeln!(f, "else:")?;
                failure.pp(f, depth)?;
                indent(f, depth)?; writeln!(f, ";;")
            },

            &Stmt::Let { ref dest, ref value } => {
                writeln!(f, "let {} = {}", dest, value)
            },

            &Stmt::Match { ref value, ref arms, ref failure } => {
                writeln!(f, "match {}:", value)?;
                for arm in arms {
                    indent(f, depth)?;
                    writeln!(f, "| {}", arm.pattern)?;
                    arm.body.pp(f, depth)?;
                }
                indent(f, depth)?;
                writeln!(f, "| else")?;
                failure.pp(f, depth)?;
                indent(f, depth)?; writeln!(f, ";;")
            },

            &Stmt::Recur { ref target } => {
                writeln!(f, "recur {}", target)
            },

            &Stmt::Return { ref result } => {
                writeln!(f, "return {:?}", result)
            },

            &Stmt::SendMsg { ref target, ref message } => {
                writeln!(f, "{} <- {}", target, message)
            },

            &Stmt::Trace { ref value } => {
                writeln!(f, "trace {}", value)
            },

            &Stmt::Wait { ref value } => {
                writeln!(f, "wait {}", value)
            },
        }
    }
}
