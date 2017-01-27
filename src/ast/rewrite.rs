use ast::*;

pub trait Rewriter<Error> {
    fn rewrite_module(&mut self, t: Module) -> Result<Module, Error> {
        Ok(Module {
            globals: each(t.globals, |t| self.rewrite_stmt(t))?,
            knots: each(t.knots, |t| self.rewrite_knot(t))?,
        })
    }

    fn rewrite_knot(&mut self, t: Knot) -> Result<Knot, Error> {
        Ok(Knot {
            args: each(t.args, |t| self.rewrite_var(t))?,
            name: self.rewrite_label(t.name)?,
            body: self.rewrite_block(t.body)?,
        })
    }

    fn rewrite_choice(&mut self, t: Choice) -> Result<Choice, Error> {
        Ok(Choice {
            guard: self.rewrite_expr(t.guard)?,
            title: self.rewrite_expr(t.title)?,
            body: self.rewrite_block(t.body)?,
        })
    }

    fn rewrite_trap(&mut self, t: Trap) -> Result<Trap, Error> {
        Ok(Trap {
            pattern: self.rewrite_pat(t.pattern)?,
            origin: self.rewrite_pat(t.origin)?,
            guard: self.rewrite_expr(t.guard)?,
            body: self.rewrite_block(t.body)?,
        })
    }

    fn rewrite_block(&mut self, t: Vec<Stmt>) -> Result<Vec<Stmt>, Error> {
        each(t, |t| self.rewrite_stmt(t))
    }

    fn rewrite_stmt(&mut self, t: Stmt) -> Result<Stmt, Error> {
        let t = match t {
            Stmt::Empty => {
                Stmt::Empty
            },

            Stmt::Disarm(label) => {
                Stmt::Disarm(self.rewrite_label(label)?)
            },

            Stmt::Let(assign, value) => {
                // Evaluation order
                let value = self.rewrite_expr(value)?;
                let assign = self.rewrite_assign(assign)?;
                Stmt::Let(assign, value)
            },

            Stmt::Listen(t) => {
                Stmt::Listen(each(t, |t| self.rewrite_trap(t))?)
            },

            Stmt::SendMsg(dst, args) => {
                let args = self.rewrite_expr(args)?;
                let dst = self.rewrite_expr(dst)?;
                Stmt::SendMsg(dst, args)
            },

            Stmt::LetSpawn(assign, fncall) => {
                // Evaluation order
                let fncall = self.rewrite_fncall(fncall)?;
                let assign = self.rewrite_assign(assign)?;
                Stmt::LetSpawn(assign, fncall)
            },

            Stmt::Recur(fncall) => {
                Stmt::Recur(self.rewrite_fncall(fncall)?)
            },

            Stmt::Trace(expr) => {
                Stmt::Trace(self.rewrite_expr(expr)?)
            },

            Stmt::Trap(label, traps) => {
                let label = self.rewrite_label(label)?;
                let traps = each(traps, |t| self.rewrite_trap(t))?;
                Stmt::Trap(label, traps)
            },

            Stmt::Wait(expr) => {
                Stmt::Wait(self.rewrite_expr(expr)?)
            },

            Stmt::Weave(label, choices) => {
                let label = self.rewrite_label(label)?;
                let choices = each(choices, |t| self.rewrite_choice(t))?;
                Stmt::Weave(label, choices)
            },
        };

        Ok(t)
    }

    fn rewrite_label(&mut self, t: Label) -> Result<Label, Error> {
        Ok(t)
    }

    fn rewrite_fncall(&mut self, t: FnCall) -> Result<FnCall, Error> {
        let label = self.rewrite_label(t.0)?;
        let args = each(t.1, |t| self.rewrite_expr(t))?;
        Ok(FnCall(label, args))
    }

    fn rewrite_assign(&mut self, t: Assign) -> Result<Assign, Error> {
        let t = match t {
            Assign::Hole => Assign::Hole,
            Assign::Var(v) => Assign::Var(self.rewrite_var(v)?),
        };

        Ok(t)
    }

    fn rewrite_var(&mut self, t: Var) -> Result<Var, Error> {
        Ok(t)
    }

    fn rewrite_pat(&mut self, t: Pat) -> Result<Pat, Error> {
        let t = match t {
            Pat::Assign(a) => {
                Pat::Assign(self.rewrite_assign(a)?)
            },

            Pat::List(l) => {
                Pat::List(each(l, |t| self.rewrite_pat(t))?)
            },

            Pat::Literal(l) => {
                Pat::Literal(self.rewrite_lit(l)?)
            },

            Pat::Match(v) => {
                Pat::Match(self.rewrite_var(v)?)
            },
        };

        Ok(t)
    }

    fn rewrite_expr(&mut self, t: Expr) -> Result<Expr, Error> {
        let t = match t {
            Expr::Str(s) => Expr::Str(s),

            Expr::Literal(l) => {
                Expr::Literal(self.rewrite_lit(l)?)
            },

            Expr::Count(label) => {
                Expr::Count(self.rewrite_label(label)?)
            },

            Expr::Var(v) => {
                Expr::Var(self.rewrite_var(v)?)
            },

            Expr::Not(b) => {
                Expr::Not(Box::new(self.rewrite_expr(*b)?))
            },

            Expr::List(v) => {
                Expr::List(each(v, |t| self.rewrite_expr(t))?)
            },

            Expr::Binop(lhs, op, rhs) => {
                let lhs = Box::new(self.rewrite_expr(*lhs)?);
                // let op = op;
                let rhs = Box::new(self.rewrite_expr(*rhs)?);
                Expr::Binop(lhs, op, rhs)
            },

            Expr::LastResort => Expr::LastResort,
        };

        Ok(t)
    }

    fn rewrite_lit(&mut self, t: Lit) -> Result<Lit, Error> {
        Ok(t)
    }
}

#[inline(always)]
pub fn each<T, E, F>(mut vec: Vec<T>, mut callback: F) -> Result<Vec<T>, E>
    where F: FnMut(T) -> Result<T, E>
{
    let mut ret = Vec::with_capacity(vec.len());
    for item in vec.drain(..) { ret.push(callback(item)?); }
    Ok(ret)
}
