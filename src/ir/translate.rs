use std::collections::HashMap;

use vecmap::CheckedFrom;

use ir;
use vm;

use driver::Try;

impl ir::Program {
    pub fn translate(self) -> Try<vm::Program> {
        let mut translator = Translator {
            registers: self.alloc_registers()?,
            code: Vec::new(),
            jump_table: vm::JumpTable::with_capacity(self.blocks.len()),
            current: vm::Label::checked_from(0).unwrap(),
        };

        for block in self.blocks.into_iter() {
            translator.tr_block(block)?;
        }

        ice!("Unimplemented");
    }

    pub fn alloc_registers(&self) -> Try<HashMap<ir::Var, vm::Reg>> {
        ice!("Unimplemented");
    }
}

struct Translator {
    registers: HashMap<ir::Var, vm::Reg>,
    code: Vec<vm::Instr>,
    jump_table: vm::JumpTable,
    current: vm::Label,
}

impl Translator {
    fn emit(&mut self, i: vm::Instr) -> Try<()> {
        ice!("Unimplemented")
    }

    fn tr_block(&mut self, t: ir::Block) -> Try<()> {
        let addr = match vm::InstrAddr::checked_from(self.code.len()) {
            Some(addr) => addr,
            None => ice!("Jump table overflow"),
        };

        match self.jump_table.push(addr) {
            Ok(_) => (),
            Err(err) => ice!("{:?}", err),
        };

        for op in t.ops.into_iter() {
            self.tr_op(op)?;
        }

        self.tr_exit(t.exit)
    }

    fn tr_op(&mut self, t: ir::Op) -> Try<()> {
        match t {
            _ => ice!("Unimplemented")
        }
    }

    fn tr_exit(&mut self, t: ir::Exit) -> Try<()> {
        match t {
            ir::Exit::EndProcess => {
                self.emit(vm::Instr::Bye)
            },

            ir::Exit::Goto(label) => {
                let label = self.tr_label(label)?;
                self.emit(vm::Instr::Jump(label))
            },

            ir::Exit::IfThenElse(flag, succ, fail) => {
                let flag = self.tr_flag(flag)?;
                let succ = self.tr_label(succ)?;
                self.emit(vm::Instr::JumpIf(succ, flag))?;
                let fail = self.tr_label(fail)?;
                self.emit(vm::Instr::Jump(fail))
            },

            ir::Exit::Recur(ir::FnCall { argv, label }) => {
                let label = self.tr_label(label)?;
                let argv = self.tr_var(argv)?;
                self.emit(vm::Instr::Recur(argv, label))
            },

            ir::Exit::Return(result) => {
                self.emit(vm::Instr::Return(result))
            },
        }
    }

    fn tr_var(&mut self, t: ir::Var) -> Try<vm::Reg> {
        match self.registers.get(&t) {
            Some(&reg) => Ok(reg),
            None => ice!("Unallocated IR var: {:?}", t),
        }
    }

    fn tr_flag(&mut self, t: ir::Flag) -> Try<vm::Flag> {
        Ok(vm::Flag(t.0))
    }

    fn tr_label(&mut self, t: ir::Label) -> Try<vm::Label> {
        Ok(vm::Label(t.0))
    }
}
