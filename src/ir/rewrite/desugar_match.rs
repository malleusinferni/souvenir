use std::collections::HashMap;

use ir::*;
use ir::rewrite::*;

struct Pass;

impl Rewriter for Pass {
    fn rw_match(&mut self, v: Expr, t: Vec<MatchArm>, e: Scope) -> Try<Stmt> {
        let mut else_clause = e.body;
        let mut arms = t;

        while let Some(arm) = arms.pop() {
            else_clause = vec!{
                Stmt::If {
                    test: unimplemented!(),
                    success: arm.body, // FIXME: Add bindings
                    failure: else_clause.into(),
                }
            }.into();
        }

        Ok(Stmt::Desugared {
            from: SugarKind::Match,
            stmts: unimplemented!(),
        })
    }
}

struct Walker {
    bindings: HashMap<String, Expr>,
    tests: Vec<Cond>,
    path: Vec<usize>,
    root: Expr,
}

impl Walker {
    fn walk_pat(&mut self, t: Pat) -> Try<()> {
        match t {
            Pat::Hole => (),

            Pat::Assign(id) => match id {
                Var::Id(name) => {
                    if self.bindings.contains_key(&name) {
                        ice!("Shadowed assignment to {} in pattern", &name);
                    }

                    let path = self.path_expr();
                    self.bindings.insert(name, path);
                },

                other => ice!("Can't assign {} in pattern", other),
            },

            Pat::EqualTo(expr) => {
                let expr = self.rw_expr(expr)?;
                let path = self.path_expr();

                // Want to write: path == expr
                self.tests.push(unimplemented!());
            },

            Pat::List(patterns) => {
                let path = self.path_expr();
                self.tests.push(Cond::HasLength(path, patterns.len() as u32));

                for (i, pattern) in patterns.into_iter().enumerate() {
                    self.path.push(i);
                    self.walk_pat(pattern)?;
                    self.path.pop();
                }
            },
        }

        Ok(())
    }

    fn path_expr(&self) -> Expr {
        let mut path = self.root.clone();
        for &index in self.path.iter().rev() {
            path = Expr::Nth(Box::new(path), index as u32);
        }
        path
    }
}

impl Rewriter for Walker {
    fn rw_var_eval(&mut self, t: Var) -> Try<Expr> {
        Ok(match t {
            Var::Id(name) => {
                self.bindings.get(&name)
                    .cloned()
                    .unwrap_or(Expr::Var(Var::Id(name)))
            },

            other => Expr::Var(other), // Unchanged
        })
    }
}
