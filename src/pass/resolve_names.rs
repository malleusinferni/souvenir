use std::collections::HashMap;

use ast::*;
use ast::rewrite::*;

#[derive(Clone, Debug)]
pub enum NameErr {
    NotFound(String),
    ScopeUnderflow,
    InvalidAst,
}

impl Module {
    pub fn resolve_names(self) -> Result<Self, NameErr> {
        let mut pass = Pass {
            env: Vec::new(),
            reg: 0,
        };

        let result = pass.rewrite_module(self);
        if cfg!(test) { Check.rewrite_module(result?) } else { result }
    }
}

struct Scope {
    bindings: HashMap<String, u32>,
    first: u32,
}

struct Pass {
    env: Vec<Scope>,
    reg: u32,
}

impl Pass {
    fn bump(&mut self) -> u32 {
        let reg = self.reg;
        self.reg += 1;
        reg
    }

    fn lookup(&self, name: &str) -> Option<u32> {
        for scope in self.env.iter().rev() {
            if let Some(value) = scope.bindings.get(name) {
                return Some(value.clone());
            }
        }

        None
    }

    fn bind(&mut self, name: &str) -> Result<u32, NameErr> {
        let value = self.bump();

        if let Some(scope) = self.env.iter_mut().last() {
            scope.bindings.insert(name.to_owned(), value);

            Ok(value)
        } else {
            Err(NameErr::ScopeUnderflow)
        }
    }

    fn enter(&mut self) {
        self.env.push(Scope {
            first: self.reg,
            bindings: HashMap::new(),
        });
    }

    fn leave<T>(&mut self, t: T) -> Result<T, NameErr> {
        if let Some(Scope { first, .. }) = self.env.pop() {
            self.reg = first;
            Ok(t)
        } else {
            Err(NameErr::ScopeUnderflow)
        }
    }
}

impl Rewriter<NameErr> for Pass {
    fn rewrite_module(&mut self, t: Module) -> Result<Module, NameErr> {
        self.enter();

        let globals = each(t.globals, |t| self.rewrite_stmt(t))?;
        let knots = each(t.knots, |t| self.rewrite_knot(t))?;

        self.leave(Module {
            globals: globals,
            knots: knots,
        })
    }

    fn rewrite_knot(&mut self, t: Knot) -> Result<Knot, NameErr> {
        self.enter();

        let args = each(t.args, |arg| {
            match arg {
                Var::Name(name) => Ok(Var::Register(self.bind(&name)?)),
                Var::Register(_) => Err(NameErr::InvalidAst),
            }
        })?;

        let t = Knot {
            args: args,
            name: self.rewrite_label(t.name)?,
            body: each(t.body, |t| self.rewrite_stmt(t))?,
        };

        self.leave(t)
    }

    fn rewrite_trap(&mut self, t: Trap) -> Result<Trap, NameErr> {
        self.enter();

        let t = Trap {
            pattern: self.rewrite_pat(t.pattern)?,
            origin: self.rewrite_pat(t.origin)?,
            guard: self.rewrite_expr(t.guard)?,
            body: each(t.body, |t| self.rewrite_stmt(t))?,
        };

        self.leave(t)
    }

    fn rewrite_block(&mut self, t: Vec<Stmt>) -> Result<Vec<Stmt>, NameErr> {
        self.enter();

        let t = each(t, |t| self.rewrite_stmt(t))?;

        self.leave(t)
    }

    fn rewrite_assign(&mut self, t: Assign) -> Result<Assign, NameErr> {
        let t = match t {
            Assign::Hole => Assign::Hole,

            Assign::Var(Var::Name(name)) => {
                // Unconditionally bind this in the local scope.
                // NOTE: Shadows previous assignments!
                let var = Var::Register(self.bind(&name)?);
                Assign::Var(var)
            },

            Assign::Var(Var::Register(_)) => return Err(NameErr::InvalidAst),
        };

        Ok(t)
    }

    fn rewrite_pat(&mut self, t: Pat) -> Result<Pat, NameErr> {
        let t = match t {
            Pat::Assign(Assign::Var(v)) => match self.rewrite_var(v) {
                Err(NameErr::NotFound(name)) => {
                    Pat::Assign(Assign::Var(Var::Register(self.bind(&name)?)))
                },

                var => Pat::Match(var?),
            },

            Pat::List(l) => {
                // Explicitly handle this so we can recurse into it
                Pat::List(each(l, |t| self.rewrite_pat(t))?)
            },

            other => other,
        };

        Ok(t)
    }

    fn rewrite_var(&mut self, t: Var) -> Result<Var, NameErr> {
        match t {
            Var::Name(name) => {
                if let Some(reg) = self.lookup(&name) {
                    Ok(Var::Register(reg))
                } else {
                    Err(NameErr::NotFound(name))
                }
            },

            Var::Register(_) => Err(NameErr::InvalidAst),
        }
    }
}

struct Check;

impl Rewriter<NameErr> for Check {
    fn rewrite_var(&mut self, t: Var) -> Result<Var, NameErr> {
        match t {
            r@Var::Register(_) => Ok(r),
            _ => Err(NameErr::InvalidAst),
        }
    }
}

#[test]
fn good() {
    let src1 = r#"
        let A = 1
        == start
        trace A
        "#;

    Module::parse(src1)
        .unwrap()
        .qualify_labels(Modpath(vec![]))
        .unwrap()
        .resolve_names()
        .unwrap();

    let src2 = r#"
        == start
        let Child = spawn util:timeout(4)
        listen
        | #ok from Any when 1
            Any <- #test
        ;;
        "#;

    Module::parse(src2)
        .unwrap()
        .qualify_labels(Modpath(vec![]))
        .unwrap()
        .resolve_names()
        .unwrap();
}

#[test]
#[should_panic]
fn evil() {
    let src = r#"
        == start
        trace OhNo
        "#;

    Module::parse(src)
        .unwrap()
        .qualify_labels(Modpath(vec![]))
        .unwrap()
        .resolve_names()
        .unwrap();
}
