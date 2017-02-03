use ir::*;
use ir::rewrite::*;

struct Pass;

impl Rewriter for Pass {
    fn rw_listen(&mut self, l: Label, t: Vec<TrapArm>) -> Try<Stmt> {
        Ok(Stmt::Desugared {
            from: SugarKind::Listen,
            stmts: vec!{
                Stmt::Sugar {
                    stmt: SugarStmt::Trap {
                        label: l,
                        arms: t,
                    },
                },
                Stmt::Wait {
                    value: Expr::Infinity,
                },
            },
        })
    }
}
