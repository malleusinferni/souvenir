use ir::*;
use ir::rewrite::*;

struct Pass;

impl Rewriter for Pass {
    fn rw_weave(&mut self, l: Label, a: Vec<WeaveArm>) -> Try<Stmt> {
        let mut stmts = Vec::with_capacity(a.len());
        let mut arms = Vec::with_capacity(a.len());

        for (i, arm) in a.into_iter().enumerate() {
            let (test, tag) = match arm.guard {
                Cond::LastResort => {
                    (Cond::True, Expr::Atom(Atom::LastResort))
                },

                test => (test, Expr::Int(i as i32)),
            };

            let message = Expr::List(vec!{
                Expr::Atom(Atom::MenuItem),
                tag,
            });

            stmts.push(Stmt::If {
                test: test,
                success: vec!{
                    Stmt::SendMsg {
                        target: Expr::PidZero,
                        message: message.clone(),
                    },
                }.into(),
                failure: vec![].into(),
            });

            arms.push(TrapArm {
                pattern: Pat::EqualTo(message),
                sender: Pat::EqualTo(Expr::PidZero),
                guard: Cond::True,
                body: arm.body,
            });
        }

        stmts.push(Stmt::Sugar {
            stmt: SugarStmt::Trap {
                label: l,
                arms: arms,
            },
        });

        // We desugar to a Trap instead of a Listen so we can avoid a race
        // condition where the response to the message might be lost

        stmts.push(Stmt::SendMsg {
            target: Expr::PidZero,
            message: Expr::List(vec![Expr::Atom(Atom::MenuEnd)]),
        });

        stmts.push(Stmt::Wait {
            value: Expr::Infinity,
        });

        Ok(Stmt::Desugared {
            from: SugarKind::Weave,
            stmts: stmts,
        })
    }
}
