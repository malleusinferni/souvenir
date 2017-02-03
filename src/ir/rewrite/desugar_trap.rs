use ir::*;
use ir::rewrite::*;

struct Pass;

impl Rewriter for Pass {
    fn rw_trap(&mut self, l: Label, a: Vec<TrapArm>) -> Try<Stmt> {
        let message_arg = Var::Gen(0);
        let sender_arg = Var::Gen(1);

        let body = vec!{
            Stmt::Let {
                dest: message_arg.clone(),
                value: Expr::FetchArgument,
            },

            Stmt::Let {
                dest: sender_arg.clone(),
                value: Expr::FetchArgument,
            },

            Stmt::Sugar {
                stmt: SugarStmt::Match {
                    value: Expr::List(vec!{
                        Expr::Var(message_arg),
                        Expr::Var(sender_arg),
                    }),

                    arms: a.into_iter().map(|arm| MatchArm {
                        pattern: Pat::List(vec!{
                            arm.pattern,
                            arm.sender,
                        }),

                        guard: arm.guard,

                        body: arm.body,
                    }).collect(),

                    failure: vec!{
                        Stmt::Return { result: false },
                    }.into(),
                },
            }
        };

        Ok(Stmt::Desugared {
            from: SugarKind::Trap,
            stmts: vec!{
                Stmt::Arm {
                    name: l,
                    body: body.into(),
                }
            },
        })
    }
}
