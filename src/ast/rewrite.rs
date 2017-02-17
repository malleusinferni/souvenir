use ast::*;
use ast::pass::*;

use driver::Try;

#[derive(Clone, Debug)]
pub struct Counter<T>(pub u32, pub fn(u32) -> T);

impl<T> Counter<T> {
    pub fn next(&mut self) -> T {
        let i = self.0;
        self.0 += 1;
        (self.1)(i)
    }
}

pub trait Rewriter {
    fn rw_desugared(&mut self, t: DesugaredProgram) -> Try<DesugaredProgram> {
        Ok(DesugaredProgram {
            preludes: each(t.preludes, |(modpath, t)| {
                Ok((modpath, self.rw_block(t)?))
            })?,
            scenes: each(t.scenes, |t| self.rw_scene(t))?,
            lambdas: each(t.lambdas, |t| self.rw_lambda(t))?,
        })
    }

    fn rw_scene(&mut self, t: Scene) -> Try<Scene> {
        Ok(Scene {
            name: self.rw_scene_name(t.name)?,
            args: each(t.args, |t| match t {
                Some(t) => Ok(Some(self.rw_id_assign(t)?)),
                None => Ok(None),
            })?,
            body: self.rw_block(t.body)?,
        })
    }

    fn rw_lambda(&mut self, t: TrapLambda) -> Try<TrapLambda> {
        Ok(TrapLambda {
            label: self.rw_label(t.label)?,
            captures: each(t.captures, |t| {
                self.rw_id_assign(t)
            })?,
            body: self.rw_block(t.body)?,
        })
    }

    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(body) = t;
        Ok(Block(each(body, |t| self.rw_stmt(t))?))
    }

    fn rw_stmt(&mut self, t: Stmt) -> Try<Stmt> {
        Ok(match t {
            Stmt::Empty => Stmt::Empty,

            Stmt::Disarm { target } => Stmt::Disarm {
                target: self.rw_label(target)?,
            },

            Stmt::Discard { value } => Stmt::Discard {
                value: self.rw_expr(value)?,
            },

            Stmt::If { test, success, failure } => Stmt::If {
                test: self.rw_cond(test)?,
                success: self.rw_block(success)?,
                failure: self.rw_block(failure)?,
            },

            Stmt::Let { value, name } => Stmt::Let {
                value: self.rw_expr(value)?,
                name: self.rw_id_assign(name)?,
            },

            Stmt::Arm { target, with_env, blocking } => Stmt::Arm {
                target: self.rw_label(target)?,
                with_env: self.rw_expr(with_env)?,
                blocking: blocking,
            },

            Stmt::Listen { name, arms } => Stmt::Listen {
                name: self.rw_label(name)?,
                arms: each(arms, |t| {
                    // self.enter()
                    let t = TrapArm {
                        pattern: self.rw_pat(t.pattern)?,
                        origin: self.rw_pat(t.origin)?,
                        guard: self.rw_cond(t.guard)?,
                        body: self.rw_block(t.body)?,
                    };
                    // self.leave()
                    Ok(t)
                })?,
            },

            Stmt::Match { value, arms, or_else } => Stmt::Match {
                value: self.rw_expr(value)?,
                arms: each(arms, |t| {
                    // self.enter()
                    let t = MatchArm {
                        pattern: self.rw_pat(t.pattern)?,
                        guard: self.rw_cond(t.guard)?,
                        body: self.rw_block(t.body)?,
                    };
                    // self.leave()
                    Ok(t)
                })?,
                or_else: self.rw_block(or_else)?,
            },

            Stmt::Naked { message, target } => Stmt::Naked {
                message: message, // FIXME: Add hook to rewrite this
                target: self.rw_expr(target)?,
            },

            Stmt::Recur { target } => Stmt::Recur {
                target: self.rw_call(target)?,
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

            Stmt::Trap { name, arms } => Stmt::Trap {
                name: self.rw_label(name)?,
                arms: each(arms, |t| {
                    // self.enter()
                    let t = TrapArm {
                        pattern: self.rw_pat(t.pattern)?,
                        origin: self.rw_pat(t.origin)?,
                        guard: self.rw_cond(t.guard)?,
                        body: self.rw_block(t.body)?,
                    };
                    // self.leave()
                    Ok(t)
                })?,
            },

            Stmt::Wait { value } => Stmt::Wait {
                value: self.rw_expr(value)?,
            },

            Stmt::Weave { name, arms } => Stmt::Weave {
                name: self.rw_label(name)?,
                arms: each(arms, |t| {
                    // Don't need to enter/leave until after we desugar this
                    let t = WeaveArm {
                        guard: self.rw_cond(t.guard)?,
                        message: self.rw_expr(t.message)?,
                        body: self.rw_block(t.body)?,
                    };
                    Ok(t)
                })?,
            },
        })
    }

    fn rw_pat(&mut self, t: Pat) -> Try<Pat> {
        Ok(match t {
            Pat::Hole => Pat::Hole,
            Pat::Assign(v) => Pat::Assign(self.rw_id_assign(v)?),
            Pat::Match(e) => Pat::Match(self.rw_expr(e)?),
            Pat::List(items) => Pat::List({
                each(items, |t| self.rw_pat(t))?
            }),
        })
    }

    fn rw_expr(&mut self, t: Expr) -> Try<Expr> {
        Ok(match t {
            Expr::Atom(a) => Expr::Atom(a),
            Expr::Int(n) => Expr::Int(n),
            Expr::Str(s) => Expr::Str(s),

            Expr::PidOfSelf => Expr::PidOfSelf,
            Expr::PidZero => Expr::PidZero,
            Expr::Infinity => Expr::Infinity,
            Expr::Arg(n) => Expr::Arg(n),

            Expr::Bool(cond) => Expr::Bool({
                Box::new(self.rw_cond(*cond)?)
            }),

            Expr::Id(v) => {
                self.rw_id_eval(v)?
            },

            Expr::MenuChoice(items) => Expr::MenuChoice({
                each(items, |t| self.rw_expr(t))?
            }),

            Expr::Nth(list, n) => Expr::Nth({
                Box::new(self.rw_expr(*list)?)
            }, n),

            Expr::Op(op, args) => Expr::Op(op, {
                each(args, |t| self.rw_expr(t))?
            }),

            Expr::List(items) => Expr::List({
                each(items, |t| self.rw_expr(t))?
            }),

            Expr::Splice(items) => Expr::Splice({
                each(items, |t| self.rw_expr(t))?
            }),

            Expr::Spawn(call) => Expr::Spawn({
                self.rw_call(call)?
            }),
        })
    }

    fn rw_cond(&mut self, t: Cond) -> Try<Cond> {
        Ok(match t {
            Cond::Not(t) => {
                let t = self.rw_cond(*t)?;
                Cond::Not(Box::new(t))
            },

            Cond::Compare(op, lhs, rhs) => {
                let lhs = self.rw_expr(lhs)?;
                let rhs = self.rw_expr(rhs)?;
                Cond::Compare(op, lhs, rhs)
            },

            Cond::HasLength(list, length) => {
                let list = self.rw_expr(list)?;
                Cond::HasLength(list, length)
            },

            Cond::True => Cond::True,
            Cond::False => Cond::False,
            Cond::LastResort => Cond::LastResort,

            Cond::And(conds) => {
                let conds = each(conds, |t| self.rw_cond(t))?;
                Cond::And(conds)
            },

            Cond::Or(conds) => {
                let conds = each(conds, |t| self.rw_cond(t))?;
                Cond::Or(conds)
            },
        })
    }

    fn rw_call(&mut self, t: Call) -> Try<Call> {
        let Call(name, args) = t;
        let name = self.rw_scene_name(name)?;
        let args = each(args, |t| self.rw_expr(t))?;
        Ok(Call(name, args))
    }

    fn rw_id_eval(&mut self, t: Ident) -> Try<Expr> {
        Ok(Expr::Id(t))
    }

    fn rw_id_assign(&mut self, t: Ident) -> Try<Ident> {
        Ok(t)
    }

    fn rw_scene_name(&mut self, t: SceneName) -> Try<SceneName> {
        Ok(t)
    }

    fn rw_label(&mut self, t: Label) -> Try<Label> {
        Ok(t)
    }
}

#[inline(always)]
pub fn each<T, F>(tree: Vec<T>, mut callback: F) -> Try<Vec<T>>
    where F: FnMut(T) -> Try<T>
{
    let mut out = Vec::with_capacity(tree.len());
    for item in tree.into_iter() { out.push(callback(item)?) }
    Ok(out)
}
