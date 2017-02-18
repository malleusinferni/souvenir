use std::collections::HashMap;

use ir::*;
use ir::visit::*;
use vm;

use driver::Try;

impl Program {
    pub fn alloc_registers(&self) -> Try<HashMap<Var, vm::Reg>> {
        let mut walker = Walker {
            allocations: HashMap::new(),
        };

        walker.visit_program(self)?;

        Ok(walker.allocations)
    }
}

struct Walker {
    allocations: HashMap<Var, vm::Reg>,
}

impl Walker {
    fn alloc(&mut self, &var: &Var) -> Try<()> {
        if self.allocations.len() >= vm::REG_COUNT {
            ice!("This program uses too many registers");
        } else {
            self.allocations.insert(var, vm::Reg(var.0 + 2));
            Ok(())
        }
    }
}

impl Visitor for Walker {
    fn visit_var_read(&mut self, var: &Var) -> Try<()> {
        self.alloc(var)
    }

    fn visit_var_write(&mut self, var: &Var) -> Try<()> {
        self.alloc(var)
    }
}
