use ast::*;

use ast::check::ICE;

pub trait Visitor {
    fn visit_program(&mut self, t: &Program) -> Result<(), ICE> {
        each(&t.modules, |&(_, ref t)| self.visit_module(t))
    }

    fn visit_module(&mut self, t: &Module) -> Result<(), ICE> {
        each(&t.globals.0, |t| self.visit_stmt(t))?;
        each(&t.knots, |t| self.visit_knot(t))
    }

    fn visit_knot(&mut self, t: &Knot) -> Result<(), ICE> {
        self.visit_fnname(&t.name)?;
        each(&t.args, |t| self.visit_ident(t))?;
        self.visit_block(&t.body)
    }

    fn visit_trap_arm(&mut self, t: &TrapArm) -> Result<(), ICE> {
        self.visit_pattern(&t.pattern)?;
        self.visit_pattern(&t.origin)?;
        self.visit_expr(&t.guard)?;
        self.visit_block(&t.body)
    }

    fn visit_weave_arm(&mut self, t: &WeaveArm) -> Result<(), ICE> {
        self.visit_expr(&t.guard)?;
        self.visit_expr(&t.message)?;
        self.visit_block(&t.body)
    }

    fn visit_fncall(&mut self, t: &FnCall) -> Result<(), ICE> {
        let &FnCall(ref name, ref args) = t;
        self.visit_fnname(name)?;
        each(args, |t| self.visit_expr(t))
    }

    fn visit_block(&mut self, t: &Block) -> Result<(), ICE> {
        let &Block(ref t) = t;
        each(t, |t| self.visit_stmt(t))
    }

    fn visit_stmt(&mut self, t: &Stmt) -> Result<(), ICE> {
        match t {
            &Stmt::Empty => {
                Ok(())
            },

            &Stmt::Disarm { ref target } => {
                self.visit_label(target)
            },

            &Stmt::Let { ref value, ref name } => {
                self.visit_expr(value)?;
                self.visit_ident(name)
            },

            &Stmt::Listen { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_trap_arm(t))
            },

            &Stmt::Naked { ref message, ref target } => {
                self.visit_string(message)?;

                if let Some(target) = target.as_ref() {
                    self.visit_ident(target)
                } else {
                    Ok(())
                }
            },

            &Stmt::Recur { ref target } => {
                self.visit_fncall(target)
            },

            &Stmt::SendMsg { ref target, ref message } => {
                self.visit_expr(message)?;
                self.visit_ident(target)
            },

            &Stmt::Trace { ref value } => {
                self.visit_expr(value)
            },

            &Stmt::Trap { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_trap_arm(t))
            },

            &Stmt::Wait { ref value } => {
                self.visit_expr(value)
            },

            &Stmt::Weave { ref name, ref arms } => {
                self.visit_label(name)?;
                each(arms, |t| self.visit_weave_arm(t))
            },
        }
    }

    fn visit_expr(&mut self, t: &Expr) -> Result<(), ICE> {
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
                self.visit_fncall(target)
            },
        }
    }

    fn visit_pattern(&mut self, t: &Pat) -> Result<(), ICE> {
        match t {
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

    fn visit_literal(&mut self, t: &Lit) -> Result<(), ICE> {
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

    fn visit_ident(&mut self, _t: &Ident) -> Result<(), ICE> {
        Ok(())
    }

    fn visit_label(&mut self, _t: &Label) -> Result<(), ICE> {
        Ok(())
    }

    fn visit_fnname(&mut self, _t: &FnName) -> Result<(), ICE> {
        Ok(())
    }

    fn visit_string(&mut self, t: &Str) -> Result<(), ICE> {
        match t {
            &Str::Plain(_) => Ok(())
        }
    }

    fn visit_atom_name(&mut self, _t: &str) -> Result<(), ICE> {
        Ok(())
    }

    fn visit_invalid_int(&mut self, _t: &str) -> Result<(), ICE> {
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
