use std::collections::HashMap;

use ir::*;
use vm;

use driver::Try;

impl Program {
    pub fn alloc_registers(&self) -> Try<HashMap<Var, vm::Reg>> {
        ice!("Unimplemented: Register allocation")
    }
}
