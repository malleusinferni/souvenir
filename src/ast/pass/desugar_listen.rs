use ast::*;
use ast::pass::*;
use ast::rewrite::*;

use driver::Try;

impl DesugaredProgram {
    pub fn desugar_listen(self) -> Try<Self> {
        Pass.rw_desugared(self)
    }
}

struct Pass;

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(source) = t;
        let mut target = Vec::with_capacity(source.len());

        for stmt in source {
            match stmt {
                Stmt::Listen { name, arms } => {
                    target.push(Stmt::Trap {
                        name: self.rw_label(name)?,
                        arms: each(arms, |t: TrapArm| Ok(TrapArm {
                            pattern: t.pattern,
                            origin: t.origin,
                            guard: t.guard,

                            body: self.rw_block(t.body)?,
                        }))?,
                    });

                    target.push(Stmt::Wait {
                        value: Expr::Infinity,
                    });
                },

                other => target.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(target))
    }
}
