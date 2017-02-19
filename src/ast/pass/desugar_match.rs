use std::collections::HashMap;

use ast::*;
use ast::pass::*;
use ast::rewrite::*;

use driver::Try;

impl DesugaredProgram {
    pub fn desugar_match(self) -> Try<Self> {
        fn gensym(id: u32) -> Ident {
            Ident {
                name: format!("Gensym%{:04X}%Match", id),
            }
        }

        let mut pass = Pass {
            gensym: Counter(0, gensym),
        };

        pass.rw_desugared(self)
    }
}

struct Pass {
    gensym: Counter<Ident>,
}

impl Pass {
    fn rw_match(&mut self, v: Ident, t: Vec<MatchArm>, e: Block) -> Try<Stmt> {
        let v = Expr::Id(v);

        let mut tail = Stmt::If {
            test: Cond::True,
            success: self.rw_block(e)?,
            failure: Block(vec![]),
        };

        for arm in t.into_iter().rev() {
            let rewriter = RwPat {
                bindings: HashMap::new(),
                tests: vec![],
                path: vec![],
                root: v.clone(),
            };

            let mut tests = rewriter.pat_to_cond(arm.pattern)?;
            tests.push(arm.guard);

            tail = Stmt::If {
                test: Cond::And(tests),
                success: self.rw_block(arm.body)?,
                failure: Block(vec![tail]),
            };
        }

        Ok(tail)
    }
}

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(input) = t;
        let mut output = Vec::with_capacity(input.len());

        // self.enter();
        for stmt in input.into_iter() {
            match stmt {
                Stmt::Match { value, arms, or_else } => {
                    let name = self.gensym.next();

                    output.push(Stmt::Let {
                        name: name.clone(),
                        value: value,
                    });

                    output.push(self.rw_match(name, arms, or_else)?);
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }
        //self.leave()?;

        Ok(Block(output))
    }
}

struct RwPat {
    bindings: HashMap<Ident, Expr>,
    tests: Vec<Cond>,
    path: Vec<usize>,
    root: Expr,
}

impl RwPat {
    fn pat_to_cond(mut self, t: Pat) -> Try<Vec<Cond>> {
        self.walk_pat(t)?;
        Ok(self.tests)
    }

    fn walk_pat(&mut self, t: Pat) -> Try<()> {
        Ok(match t {
            Pat::Hole => (),

            Pat::Assign(id) => {
                if self.bindings.contains_key(&id) {
                    ice!("Shadowed assignment to {} in pattern", &id);
                }
            },

            Pat::Match(expr) => {
                let expr = self.rw_expr(expr)?;
                let path = self.path_expr();
                self.tests.push(Cond::Compare(BoolOp::Eql, expr, path));
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
        })
    }

    fn path_expr(&self) -> Expr {
        let mut root = self.root.clone();
        for &i in self.path.iter() {
            root = Expr::Nth(Box::new(root), i as u32);
        }
        root
    }
}

impl Rewriter for RwPat {
    fn rw_id_eval(&mut self, t: Ident) -> Try<Expr> {
        if let Some(expr) = self.bindings.get(&t).cloned() {
            Ok(expr)
        } else {
            Ok(Expr::Id(t))
        }
    }
}
