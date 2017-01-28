use ast::*;
use ast::rewrite::*;

impl Module {
    pub fn forbid_nested_traps(self) -> Result<Self, TrapErr> {
        let mut pass = Pass { in_trap: false };
        pass.rewrite_module(self)
    }
}

pub enum TrapErr {
    No,
}

struct Pass {
    in_trap: bool,
}

impl Rewriter<TrapErr> for Pass {
    fn rewrite_trap(&mut self, t: Trap) -> Result<Trap, TrapErr> {
        if self.in_trap { return Err(TrapErr::No); }

        self.in_trap = true;

        let t = Trap {
            pattern: self.rewrite_pat(t.pattern)?,
            origin: self.rewrite_pat(t.origin)?,
            guard: self.rewrite_expr(t.guard)?,
            body: self.rewrite_block(t.body)?,
        };

        self.in_trap = false;

        Ok(t)
    }
}
