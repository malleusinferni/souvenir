use ast::*;
use ast::rewrite::*;

#[derive(Clone, Debug)]
pub enum StmtErr {
    IoInPrelude(Stmt),
}

impl Module {
    pub fn forbid_prelude_io(self) -> Result<Self, StmtErr> {
        let mut pass = Pass;

        pass.rewrite_module(self)
    }
}

struct Pass;

impl Rewriter<StmtErr> for Pass {
    fn rewrite_module(&mut self, t: Module) -> Result<Module, StmtErr> {
        Ok(Module {
            globals: each(t.globals, |t| self.rewrite_stmt(t))?,
            knots: t.knots, // Unmodified
        })
    }

    fn rewrite_stmt(&mut self, t: Stmt) -> Result<Stmt, StmtErr> {
        let t = match t {
            Stmt::Empty => Stmt::Empty,

            Stmt::Let(pat, expr) => Stmt::Let(pat, expr),

            //Stmt::LetSpawn() // Not yet...

            other => return Err(StmtErr::IoInPrelude(other)),
        };

        Ok(t)
    }
}
