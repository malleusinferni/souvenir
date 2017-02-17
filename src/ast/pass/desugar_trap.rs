use std::collections::HashMap;

use ast::*;
use ast::pass::*;
use ast::rewrite::Rewriter;
use ast::visit::Visitor;

use driver::{Try, ErrCtx};

impl DesugaredProgram {
    pub fn desugar_trap(self) -> Try<Self> {
        use ast::rewrite::each;

        let mut pass = Pass {
            lambdas: self.lambdas,
        };

        Ok(DesugaredProgram {
            preludes: each(self.preludes, |(modpath, t)| {
                Ok((modpath, pass.rw_block(t)?))
            })?,
            scenes: each(self.scenes, |t| pass.rw_scene(t))?,
            lambdas: pass.lambdas,
        })
    }
}

struct Pass {
    lambdas: Vec<TrapLambda>,
}

impl Pass {
    fn rw_trap(&mut self, l: Label, a: Vec<TrapArm>) -> Try<Stmt> {
        let body = Block(vec!{
            Stmt::Match {
                value: Expr::List(vec!{
                    Expr::Arg(0),
                    Expr::Arg(1),
                }),

                arms: a.into_iter().map(|arm| MatchArm {
                    pattern: Pat::List(vec!{
                        arm.pattern,
                        arm.origin,
                    }),

                    guard: arm.guard,
                    body: arm.body,
                }).collect(),

                or_else: Block(vec![ Stmt::Return { result: false } ]),
            },
        });

        let mut lambda = TrapLambda {
            label: l.clone(),
            captures: vec![],
            body: body,
        };

        let captures = lambda.find_captures()?;

        self.lambdas.push(lambda);

        Ok(Stmt::Arm {
            target: l,
            with_env: captures,
            blocking: false,
        })
    }
}

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(input) = t;
        let mut output = Vec::with_capacity(input.len());

        for stmt in input {
            match stmt {
                Stmt::Trap { name, arms } => {
                    output.push(self.rw_trap(name, arms)?);
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}

impl TrapLambda {
    fn find_captures(&mut self) -> Try<Expr> {
        // FIXME: This is a hack
        let QfdLabel { in_scene, .. } = self.label.qualified()?;
        let ctx = ErrCtx::Local(in_scene, vec![]);

        let mut capturer = Capturer {
            context: ctx,
            bindings: vec![],
            captures: HashMap::new(),
        };

        capturer.visit_block(&self.body)?;

        let mut capture_exprs = Vec::with_capacity(capturer.captures.len());

        for (id, ()) in capturer.captures.into_iter() {
            self.captures.push(id.clone());
            capture_exprs.push(Expr::Id(id));
        }

        Ok(Expr::List(capture_exprs))
    }
}

struct Capturer {
    context: ErrCtx,
    bindings: Vec<HashMap<Ident, ()>>,
    captures: HashMap<Ident, ()>,
}

impl Capturer {
    fn lookup(&self, id: &Ident) -> bool {
        for scope in self.bindings.iter() {
            if scope.contains_key(id) { return true; }
        }

        self.captures.contains_key(id)
    }
}

impl Visitor for Capturer {
    fn visit_id_eval(&mut self, t: &Ident) -> Try<()> {
        if self.lookup(t) { return Ok(()); }

        let t = t.clone();
        self.captures.insert(t, ());
        Ok(())
    }

    fn visit_id_assign(&mut self, t: &Ident) -> Try<()> {
        if let Some(scope) = self.bindings.iter_mut().last() {
            scope.insert(t.clone(), ());
            Ok(())
        } else {
            ice!("Scope underflow");
        }
    }

    fn enter(&mut self) {
        self.bindings.push(HashMap::new());
    }

    fn leave(&mut self) -> Try<()> {
        if self.bindings.pop().is_none() {
            ice!("Scope underflow");
        } else {
            Ok(())
        }
    }

    fn error_context(&mut self) -> &mut ErrCtx {
        &mut self.context
    }
}
