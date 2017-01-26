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
            name: self.rewrite_label(t.name)?,
            args: each(t.args, |t| self.rewrite_expr(t))?,
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
            pattern: self.rewrite_expr(t.pattern)?,
            guard: self.rewrite_expr(t.guard)?,
            origin: self.rewrite_expr(t.origin)?,
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

            Stmt::Let(name, value) => {
                let name = self.rewrite_expr(name)?;
                let value = self.rewrite_expr(value)?;
                Stmt::Let(name, value)
            },

            Stmt::Listen(t) => {
                Stmt::Listen(each(t, |t| self.rewrite_trap(t))?)
            },

            Stmt::SendMsg(dst, args) => {
                Stmt::SendMsg(self.rewrite_expr(dst)?, self.rewrite_expr(args)?)
            },

            Stmt::LetSpawn(name, label, args) => {
                let name = self.rewrite_expr(name)?;
                let label = self.rewrite_label(label)?;
                let args = each(args, |t| self.rewrite_expr(t))?;
                Stmt::LetSpawn(name, label, args)
            },

            Stmt::TailCall(label, args) => {
                let label = self.rewrite_label(label)?;
                let args = each(args, |t| self.rewrite_expr(t))?;
                Stmt::TailCall(label, args)
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

    fn rewrite_expr(&mut self, t: Expr) -> Result<Expr, Error> {
        Ok(t)
    }
}

#[inline(always)]
fn each<T, E, F>(mut vec: Vec<T>, mut callback: F) -> Result<Vec<T>, E>
    where F: FnMut(T) -> Result<T, E>
{
    let mut ret = Vec::with_capacity(vec.len());
    for item in vec.drain(..) { ret.push(callback(item)?); }
    Ok(ret)
}
