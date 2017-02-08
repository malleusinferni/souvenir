use ast::*;
use ast::rewrite::*;

use driver::Try;

struct Pass;

impl Pass {
    fn rw_weave(&mut self, l: Label, a: Vec<WeaveArm>) -> Try<Vec<Stmt>> {
        let mut stmts = Vec::with_capacity(a.len() + 4); // Or whatever
        let mut arms = Vec::with_capacity(a.len());

        // FIXME: This will deadlock if we use nested weaves.
        // Find another way to express blocking IO here.

        for (i, arm) in a.into_iter().enumerate() {
            let (test, tag) = match arm.guard {
                Cond::LastResort => (Cond::True, Expr::Atom(Atom::LastResort)),
                other => (other, Expr::Int(i as i32)),
            };

            let message = Expr::List(vec![ Expr::Atom(Atom::MenuItem), tag ]);

            stmts.push(Stmt::If {
                test: test,
                success: Block(vec!{
                    Stmt::SendMsg {
                        target: Expr::PidZero,
                        message: message.clone(),
                    },
                }),
                failure: Block(vec![]),
            });

            arms.push(TrapArm {
                pattern: Pat::Match(message),
                origin: Pat::Match(Expr::PidZero),
                guard: Cond::True,
                body: arm.body,
            });
        }

        stmts.push(Stmt::Trap {
            name: l,
            arms: arms,
        });

        stmts.push(Stmt::SendMsg {
            target: Expr::PidZero,
            message: Expr::List(vec![Expr::Atom(Atom::MenuEnd)]),
        });

        stmts.push(Stmt::Wait {
            value: Expr::Infinity,
        });

        Ok(stmts)
    }
}

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(input) = t;
        let mut output = Vec::with_capacity(input.len());

        for stmt in input.into_iter() {
            match stmt {
                Stmt::Weave { name, arms } => {
                    output.extend(self.rw_weave(name, arms)?);
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}
