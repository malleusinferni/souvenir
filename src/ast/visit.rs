use ast::*;

use driver::{Try, ErrCtx};

pub trait Visitor {
    fn error_context(&mut self) -> &mut ErrCtx;

    fn visit_program(&mut self, t: &Program) -> Try<()> {
        each(&t.modules, |&(ref modpath, ref t)| {
            self.visit_module(t, modpath)
        })
    }

    fn visit_module(&mut self, t: &Module, p: &Modpath) -> Try<()> {
        self.error_context().begin_module(p);
        each(&t.globals.0, |t| self.visit_stmt(t))?;
        each(&t.scenes, |t| self.visit_scene(t))
    }

    fn visit_scene(&mut self, t: &Scene) -> Try<()> {
        self.error_context().begin_scene(&t.name.name)?;
        self.visit_scene_name(&t.name)?;
        each(&t.args, |t| self.visit_ident(t))?;
        self.visit_block(&t.body)?;
        self.error_context().pop()
    }

    fn visit_trap_arm(&mut self, t: &TrapArm) -> Try<()> {
        self.visit_pattern(&t.pattern)?;
        self.visit_pattern(&t.origin)?;
        self.visit_cond(&t.guard)?;
        self.visit_block(&t.body)
    }

    fn visit_weave_arm(&mut self, t: &WeaveArm) -> Try<()> {
        self.visit_cond(&t.guard)?;
        self.visit_expr(&t.message)?;
        self.visit_block(&t.body)
    }

    fn visit_call(&mut self, t: &Call) -> Try<()> {
        let &Call(ref name, ref args) = t;
        self.visit_scene_name(name)?;
        each(args, |t| self.visit_expr(t))
    }

    fn visit_block(&mut self, t: &Block) -> Try<()> {
        let &Block(ref t) = t;
        each(t, |t| self.visit_stmt(t))
    }

    fn visit_stmt(&mut self, t: &Stmt) -> Try<()> {
        self.error_context().push_stmt(t)?;

        match t {
            &Stmt::Empty => (),

            &Stmt::Disarm { ref target } => {
                self.visit_label(target)?;
            },

            &Stmt::Discard { ref value } => {
                self.visit_expr(value)?;
            },

            &Stmt::Let { ref value, ref name } => {
                self.visit_expr(value)?;
                self.visit_ident(name)?;
            },

            &Stmt::Listen { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_trap_arm(t))?;
            },

            &Stmt::Naked { ref message, ref target } => {
                self.visit_string(message)?;

                if let Some(target) = target.as_ref() {
                    self.visit_ident(target)?;
                }
            },

            &Stmt::Recur { ref target } => {
                self.visit_call(target)?;
            },

            &Stmt::SendMsg { ref target, ref message } => {
                self.visit_expr(message)?;
                self.visit_ident(target)?;
            },

            &Stmt::Trace { ref value } => {
                self.visit_expr(value)?;
            },

            &Stmt::Trap { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_trap_arm(t))?;
            },

            &Stmt::Wait { ref value } => {
                self.visit_expr(value)?;
            },

            &Stmt::Weave { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_weave_arm(t))?;
            },
        };

        self.error_context().pop()
    }

    fn visit_expr(&mut self, t: &Expr) -> Try<()> {
        match t {
            &Expr::Id(ref ident) => {
                self.visit_ident(ident)
            },

            &Expr::Lit(ref lit) => {
                self.visit_literal(lit)
            },

            &Expr::Str(ref string) => {
                self.visit_string(string)
            },

            &Expr::Op(_, ref args) => {
                each(args, |t| self.visit_expr(t))
            },

            &Expr::List(ref elems) => {
                each(elems, |t| self.visit_expr(t))
            },

            &Expr::Spawn(ref target) => {
                self.visit_call(target)
            },
        }
    }

    fn visit_cond(&mut self, t: &Cond) -> Try<()> {
        Ok(match t {
            &Cond::True => (),
            &Cond::False => (),
            &Cond::LastResort => (),

            &Cond::Compare(ref op, ref lhs, ref rhs) => {
                self.visit_expr(lhs)?;
                self.visit_expr(rhs)?;
            },

            &Cond::Not(ref cond) => {
                self.visit_cond(cond)?;
            },
        })
    }

    fn visit_pattern(&mut self, t: &Pat) -> Try<()> {
        match t {
            &Pat::Hole => Ok(()),

            &Pat::Id(ref ident) => {
                self.visit_ident(ident)
            },

            &Pat::Lit(ref literal) => {
                self.visit_literal(literal)
            },

            &Pat::List(ref list) => {
                each(list, |t| self.visit_pattern(t))
            },
        }
    }

    fn visit_literal(&mut self, t: &Lit) -> Try<()> {
        match t {
            &Lit::Atom(ref name) => {
                self.visit_atom_name(name)
            },

            &Lit::Int(_) => {
                Ok(())
            },

            &Lit::InvalidInt(ref n) => {
                self.visit_invalid_int(n)
            },
        }
    }

    fn visit_ident(&mut self, _t: &Ident) -> Try<()> {
        Ok(())
    }

    fn visit_label(&mut self, _t: &Label) -> Try<()> {
        Ok(())
    }

    fn visit_scene_name(&mut self, _t: &SceneName) -> Try<()> {
        Ok(())
    }

    fn visit_string(&mut self, t: &Str) -> Try<()> {
        match t {
            &Str::Plain(_) => Ok(())
        }
    }

    fn visit_atom_name(&mut self, _t: &str) -> Try<()> {
        Ok(())
    }

    fn visit_invalid_int(&mut self, _t: &str) -> Try<()> {
        Ok(())
    }
}

#[inline(always)]
pub fn each<T, E, F>(tree: &[T], mut callback: F) -> Result<(), E>
    where F: FnMut(&T) -> Result<(), E>
{
    for item in tree.iter() { callback(item)?; }

    Ok(())
}
