use std::fmt::{Display, Error, Formatter, Write};

use ir::*;

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "#{}", self.name())
    }
}

impl Atom {
    pub fn name(&self) -> &str {
        match self {
            &Atom::User(ref s) => s.as_ref(),
            &Atom::MenuItem => "[menu item]",
            &Atom::MenuEnd => "[show menu]",
            &Atom::LastResort => "[last resort]",
            &Atom::PrintLine => "[print line]",
            &Atom::PrintFinished => "[print finished]",
        }
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
            &Expr::Var(ref v) => write!(f, "{}", v),
            &Expr::Int(n) => write!(f, "{}", n),
            &Expr::List(ref items) => {
                write!(f, "[")?; items.pp_slice(f)?; write!(f, "]")
            },
            &Expr::Nth(ref list, n) => {
                write!(f, "({})[{}]", list, n)
            },
            &Expr::Spawn(ref fncall) => write!(f, "spawn {}", fncall),
            &Expr::Strcat(ref items) => write!(f, "{:?}", strcat(items)),
            &Expr::Strlit(ref s) => write!(f, "> {}", s),
            &Expr::PidOfSelf => write!(f, "Self"),
            &Expr::PidZero => write!(f, "%stdio"),
            &Expr::Infinity => write!(f, "forever"),
            &Expr::FetchArgument => write!(f, "$ARGUMENT"),
        }
    }
}

impl Display for Cond {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Cond::True => write!(f, "#true"),

            &Cond::False => write!(f, "#false"),

            &Cond::LastResort => write!(f, "#[last resort]"),

            &Cond::Not(ref cond) => {
                write!(f, "not {}", cond)
            },

            &Cond::HasLength(ref list, ref len) => {
                write!(f, "len({}) == {}", list, len)
            },

            &Cond::Equals(ref lhs, ref rhs) => {
                write!(f, "{} == {}", lhs, rhs)
            },

            &Cond::AllOf(ref tests) => {
                write!(f, "({})", tests.iter().map(|test| {
                    format!("{}", test)
                }).collect::<Vec<_>>().join(" and "))
            },

            &Cond::AnyOf(ref tests) => {
                write!(f, "({})", tests.iter().map(|test| {
                    format!("{}", test)
                }).collect::<Vec<_>>().join(" or "))
            },
        }
    }
}

impl Display for FnCall {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let &FnCall { name: FnId(id), ref args } = self;
        write!(f, ":{}: (", id)?;
        args.pp_slice(f)?;
        write!(f, ")")
    }
}

impl Display for Var {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Var::Reg(n) => write!(f, "%{:0x}", n),
            &Var::Gen(n) => write!(f, "Gensym[{:0x}]", n),
            &Var::Id(ref name) => write!(f, "{}", name),
        }
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
            &Stmt::Sugar { ref stmt } => {
                stmt.pp(f, depth)
            },

            &Stmt::Desugared { ref stmts, .. } => {
                for stmt in stmts.iter() {
                    stmt.pp(f, depth)?;
                }

                Ok(())
            },

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

impl SugarStmt {
    pub fn pp(&self, f: &mut Formatter, depth: u32) -> Result<(), Error> {
        match self {
            &SugarStmt::Listen { ref label, ref arms } => {
                writeln!(f, "listen {}:", label)?;
                for arm in arms {
                    indent(f, depth)?;
                    writeln!(f, "| {} from {} when {}", arm.pattern, arm.sender, arm.guard)?;
                    arm.body.pp(f, depth)?;
                }
                indent(f, depth)?; writeln!(f, ";;")
            },

            &SugarStmt::Match { ref value, ref arms, ref failure } => {
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

            &SugarStmt::Naked { .. } => {
                writeln!(f, "> UNIMPLEMENTED")
            },

            &SugarStmt::Trap { .. } => {
                writeln!(f, "trap (unimplemented)")
            },

            _ => unimplemented!(),
        }
    }
}
