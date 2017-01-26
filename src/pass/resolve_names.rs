use std::collections::HashMap;

use ast::*;
use ast::rewrite::*;

#[derive(Clone, Debug)]
pub enum NameErr {
    NotFound(String),
    ScopeUnderflow,
}

impl Module {
    pub fn resolve_names(self) -> Result<Self, NameErr> {
        let mut pass = Pass {
            env: Vec::new(),
        };

        pass.rewrite_module(self)
    }
}

struct Pass {
    env: Vec<HashMap<String, ()>>,
}

impl Pass {
    fn lookup(&self, name: &str) -> bool {
        for scope in self.env.iter().rev() {
            if scope.contains_key(name) { return true; }
        }

        false
    }

    fn bind(&mut self, name: &str, value: ()) -> Result<(), NameErr> {
        if let Some(scope) = self.env.iter_mut().last() {
            scope.insert(name.to_owned(), value);

            Ok(())
        } else {
            Err(NameErr::ScopeUnderflow)
        }
    }

    fn enter(&mut self) {
        self.env.push(HashMap::new());
    }

    fn leave<T>(&mut self, t: T) -> Result<T, NameErr> {
        if let Some(_scope) = self.env.pop() {
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

        for &Var(ref name) in t.args.iter() {
            self.bind(name, ())?;
        }

        let t = Knot {
            args: t.args,
            name: self.rewrite_label(t.name)?,
            body: each(t.body, |t| self.rewrite_stmt(t))?,
        };

        self.leave(t)
    }

    fn rewrite_block(&mut self, t: Vec<Stmt>) -> Result<Vec<Stmt>, NameErr> {
        self.enter();

        let t = each(t, |t| self.rewrite_stmt(t))?;

        self.leave(t)
    }

    fn rewrite_bind(&mut self, t: Bind) -> Result<Bind, NameErr> {
        let t = match t {
            Bind::Var(v) => {
                // NOTE: Does not recurse!
                if self.lookup(&v.0) {
                    Bind::Match(v)
                } else {
                    self.bind(&v.0, ())?;
                    Bind::Var(v)
                }
            },

            Bind::List(l) => {
                // Explicitly handle this so we can recurse into it
                Bind::List(each(l, |t| self.rewrite_bind(t))?)
            },

            other => other,
        };

        Ok(t)
    }

    fn rewrite_var(&mut self, t: Var) -> Result<Var, NameErr> {
        let Var(name) = t;

        if self.lookup(&name) {
            Ok(Var(name))
        } else {
            Err(NameErr::NotFound(name))
        }
    }
}

#[test]
fn good() {
    let src = r#"
        let A = 1
        == start
        trace A
        "#;

    Module::parse(src)
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
