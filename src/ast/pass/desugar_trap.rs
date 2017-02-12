use ast::*;
use ast::rewrite::*;

use driver::Try;

impl Program {
    pub fn desugar_trap(self) -> Try<Self> {
        (Pass { next_sym: 0 }).rw_program(self)
    }
}

struct Pass {
    next_sym: u32,
}

impl Pass {
    fn gensym(&mut self) -> Ident {
        let sym = self.next_sym;
        self.next_sym += 1;
        Ident { name: format!("GENSYM%%{:X}", sym) }
    }

    fn rw_trap(&mut self, l: Label, a: Vec<TrapArm>) -> Try<Vec<Stmt>> {
        let message_arg = self.gensym();
        let sender_arg = self.gensym();

        let body = vec!{
            Stmt::Let {
                name: message_arg.clone(),
                value: Expr::Arg,
            },

            Stmt::Let {
                name: sender_arg.clone(),
                value: Expr::Arg,
            },

            Stmt::Match {
                value: Expr::List(vec!{
                    Expr::Id(message_arg),
                    Expr::Id(sender_arg),
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
        };

        let _ = l;
        let _ = body;
        ice!("Not yet implemented");
    }
}

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(input) = t;
        let mut output = Vec::with_capacity(input.len());

        for stmt in input {
            match stmt {
                Stmt::Trap { name, arms } => {
                    output.extend(self.rw_trap(name, arms)?);
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}
