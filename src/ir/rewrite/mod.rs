pub mod desugar_weave;
pub mod desugar_listen;
pub mod desugar_trap;
pub mod desugar_match;

use ir::*;

pub enum Error {
    ICE(String),
}

pub type Try<T> = Result<T, Error>;

pub trait Rewriter {
    fn rw_program(&mut self, t: Program) -> Try<Program> {
        Ok(t)
    }

    fn rw_knot(&mut self, t: KnotDef) -> Try<KnotDef> {
        Ok(KnotDef {
            prelude_id: t.prelude_id,
            args_wanted: t.args_wanted,
            body: self.rw_scope(t.body)?,
        })
    }

    fn rw_scope(&mut self, t: Scope) -> Try<Scope> {
        Ok(Scope {
            body: each(t.body, |t| self.rw_stmt(t))?,
        })
    }

    fn rw_stmt(&mut self, t: Stmt) -> Try<Stmt> {
        Ok(match t {
            Stmt::Desugared { from, stmts } => Stmt::Desugared {
                from: from,
                stmts: each(stmts, |t| self.rw_stmt(t))?,
            },

            Stmt::Sugar { stmt } => {
                self.rw_sugar(stmt)?
            },

            Stmt::Arm { name, body } => Stmt::Arm {
                name: self.rw_label(name)?,
                body: self.rw_scope(body)?,
            },

            Stmt::Disarm { name } => Stmt::Disarm {
                name: self.rw_label(name)?,
            },

            Stmt::Discard { value } => Stmt::Discard {
                value: self.rw_expr(value)?,
            },

            Stmt::If { test, success, failure } => Stmt::If {
                test: self.rw_expr(test)?,
                success: self.rw_scope(success)?,
                failure: self.rw_scope(failure)?,
            },

            Stmt::Let { value, dest } => Stmt::Let {
                value: self.rw_expr(value)?,
                dest: self.rw_var_assign(dest)?,
            },

            Stmt::Recur { target } => Stmt::Recur {
                target: self.rw_fncall(target)?,
            },

            Stmt::Return { result } => Stmt::Return {
                result: result,
            },

            Stmt::SendMsg { target, message } => Stmt::SendMsg {
                target: self.rw_expr(target)?,
                message: self.rw_expr(message)?,
            },

            Stmt::Trace { value } => Stmt::Trace {
                value: self.rw_expr(value)?,
            },

            Stmt::Wait { value } => Stmt::Wait {
                value: self.rw_expr(value)?,
            },
        })
    }

    fn rw_sugar(&mut self, t: SugarStmt) -> Try<Stmt> {
        Ok(match t {
            SugarStmt::Listen { label, arms } => {
                self.rw_listen(label, arms)?
            },
            SugarStmt::Match { value, arms, failure } => {
                self.rw_match(value, arms, failure)?
            },
            SugarStmt::Naked { target, topic, text } => {
                self.rw_naked(target, topic, text)?
            },
            SugarStmt::Trap { label, arms } => {
                self.rw_trap(label, arms)?
            },
            SugarStmt::Weave { label, arms } => {
                self.rw_weave(label, arms)?
            },
        })
    }

    fn rw_listen(&mut self, l: Label, t: Vec<TrapArm>) -> Try<Stmt> {
        Ok(Stmt::Sugar {
            stmt: SugarStmt::Listen {
                label: self.rw_label(l)?,
                arms: each(t, |t| {
                    Ok(TrapArm {
                        pattern: self.rw_pat(t.pattern)?,
                        sender: self.rw_pat(t.sender)?,
                        guard: self.rw_expr(t.guard)?,
                        body: self.rw_scope(t.body)?,
                    })
                })?,
            },
        })
    }

    fn rw_match(&mut self, v: Expr, t: Vec<MatchArm>, e: Scope) -> Try<Stmt> {
        Ok(Stmt::Sugar {
            stmt: SugarStmt::Match {
                value: self.rw_expr(v)?,
                arms: each(t, |t| {
                    Ok(MatchArm {
                        pattern: self.rw_pat(t.pattern)?,
                        guard: self.rw_expr(t.guard)?,
                        body: self.rw_scope(t.body)?,
                    })
                })?,
                failure: self.rw_scope(e)?,
            }
        })
    }

    fn rw_naked(&mut self, d: Expr, t: Option<Expr>, m: Vec<Expr>) -> Try<Stmt> {
        Ok(Stmt::Sugar {
            stmt: SugarStmt::Naked {
                target: self.rw_expr(d)?,
                topic: match t {
                    Some(t) => Some(self.rw_expr(t)?),
                    None => None,
                },
                text: each(m, |t| self.rw_expr(t))?,
            },
        })
    }

    fn rw_trap(&mut self, l: Label, a: Vec<TrapArm>) -> Try<Stmt> {
        Ok(Stmt::Sugar {
            stmt: SugarStmt::Trap {
                label: self.rw_label(l)?,
                arms: each(a, |t| {
                    Ok(TrapArm {
                        pattern: self.rw_pat(t.pattern)?,
                        sender: self.rw_pat(t.sender)?,
                        guard: self.rw_expr(t.guard)?,
                        body: self.rw_scope(t.body)?,
                    })
                })?,
            },
        })
    }

    fn rw_weave(&mut self, l: Label, a: Vec<WeaveArm>) -> Try<Stmt> {
        Ok(Stmt::Sugar {
            stmt: SugarStmt::Weave {
                label: self.rw_label(l)?,
                arms: each(a, |t| {
                    Ok(WeaveArm {
                        guard: self.rw_expr(t.guard)?,
                        message: self.rw_expr(t.message)?,
                        body: self.rw_scope(t.body)?,
                    })
                })?,
            },
        })
    }

    fn rw_pat(&mut self, t: Pat) -> Try<Pat> {
        Ok(match t {
            Pat::Hole => Pat::Hole,
            Pat::Assign(v) => Pat::Assign(self.rw_var_assign(v)?),
            Pat::EqualTo(e) => Pat::EqualTo(self.rw_expr(e)?),
            Pat::List(items) => Pat::List({
                each(items, |t| self.rw_pat(t))?
            }),
        })
    }

    fn rw_expr(&mut self, t: Expr) -> Try<Expr> {
        Ok(match t {
            Expr::Atom(a) => Expr::Atom(a),
            Expr::Int(n) => Expr::Int(n),
            Expr::Strlit(s) => Expr::Strlit(s),
            Expr::PidOfSelf => Expr::PidOfSelf,
            Expr::PidZero => Expr::PidZero,
            Expr::Infinity => Expr::Infinity,
            Expr::FetchArgument => Expr::FetchArgument,

            Expr::Var(v) => Expr::Var({
                self.rw_var_eval(v)?
            }),

            Expr::List(items) => Expr::List({
                each(items, |t| self.rw_expr(t))?
            }),

            Expr::Strcat(items) => Expr::Strcat({
                each(items, |t| self.rw_expr(t))?
            }),

            Expr::Spawn(fncall) => Expr::Spawn({
                self.rw_fncall(fncall)?
            }),
        })
    }

    fn rw_fncall(&mut self, t: FnCall) -> Try<FnCall> {
        Ok(FnCall {
            name: t.name,
            args: each(t.args, |t| self.rw_expr(t))?,
        })
    }

    fn rw_var_eval(&mut self, t: Var) -> Try<Var> {
        Ok(t)
    }

    fn rw_var_assign(&mut self, t: Var) -> Try<Var> {
        Ok(t)
    }

    fn rw_label(&mut self, t: Label) -> Try<Label> {
        Ok(t)
    }
}

#[inline(always)]
pub fn each<T, F>(tree: Vec<T>, mut callback: F) -> Result<Vec<T>, Error>
    where F: FnMut(T) -> Result<T, Error>
{
    let mut out = Vec::with_capacity(tree.len());
    for item in tree.into_iter() { out.push(callback(item)?) }
    Ok(out)
}
