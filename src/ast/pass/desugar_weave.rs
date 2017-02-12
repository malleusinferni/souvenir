use ast::*;
use ast::rewrite::*;

use driver::Try;

impl Program {
    pub fn desugar_weave(self) -> Try<Self> {
        Pass.rw_program(self)
    }
}

struct Pass;

impl Pass {
    fn rw_weave(&mut self, l: Label, a: Vec<WeaveArm>) -> Try<Stmt> {
        let mut choices = Vec::with_capacity(a.len());
        let mut arms = Vec::with_capacity(a.len());

        let mut or_else = Block(vec![]);

        for (i, arm) in a.into_iter().enumerate() {
            let choice = match arm.guard {
                Cond::LastResort => {
                    or_else = arm.body;
                    continue;
                },

                other => vec![
                    Expr::Bool(Box::new(other)),
                    Expr::Int(i as i32),
                    arm.message,
                ],
            };

            arms.push(MatchArm {
                pattern: Pat::Match(Expr::Int(i as i32)),
                guard: Cond::True,
                body: self.rw_block(arm.body)?,
            });
        }

        // FIXME: Do we still want weaves to have labels at all?
        let _ = l;

        Ok(Stmt::Match {
            value: Expr::MenuChoice(choices),
            arms: arms,
            or_else: or_else,
        })
    }
}

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(input) = t;
        let mut output = Vec::with_capacity(input.len());

        for stmt in input.into_iter() {
            match stmt {
                Stmt::Weave { name, arms } => {
                    output.push(self.rw_weave(name, arms)?);
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}
