use std::collections::HashMap;

use string_interner::StringInterner;

use vecmap::CheckedFrom;

use ir;
use vm;

use driver::Try;

impl ir::Program {
    pub fn translate(self) -> Try<vm::Program> {
        let mut translator = Translator {
            registers: self.alloc_registers()?,
            env_table: self.build_env_table()?,
            code: Vec::new(),
            jump_table: vm::JumpTable::with_capacity(self.blocks.len()),
            str_table: self.str_table,
            atom_table: self.atom_table,
            current: vm::Label::checked_from(0).unwrap(),
        };

        for block in self.blocks.into_iter() {
            translator.tr_block(block)?;
        }

        Ok(vm::Program {
            code: translator.code.into(),
            jump_table: translator.jump_table,
            str_table: translator.str_table,
            atom_table: translator.atom_table,
            //env_table: translator.env_table,
        })
    }

    pub fn build_env_table(&self) -> Try<HashMap<ir::Label, vm::EnvId>> {
        //ice!("Unimplemented: Env table construction");
        Ok(HashMap::new())
    }
}

struct Translator {
    registers: HashMap<ir::Var, vm::Reg>,
    env_table: HashMap<ir::Label, vm::EnvId>,
    code: Vec<vm::Instr>,
    jump_table: vm::JumpTable,
    str_table: StringInterner<vm::StrId>,
    atom_table: StringInterner<vm::AtomId>,
    current: vm::Label,
}

impl Translator {
    fn emit(&mut self, i: vm::Instr) -> Try<()> {
        self.code.push(i);
        Ok(())
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
        type Binop = fn(vm::Reg, vm::Reg) -> vm::Instr;

        let tr_binop = |this: &mut Self, op: Binop, l, r, dst| {
            let l = this.tr_var(l)?;
            let r = this.tr_var(r)?;
            let dst = this.tr_var(dst)?;
            if l != r && l != dst {
                this.emit(vm::Instr::Cpy(l, dst))?;
            }
            this.emit(op(r, dst))
        };

        match t {
            ir::Op::Arm(trap_ref) => {
                let env = self.tr_var(trap_ref.env)?;
                let label = self.tr_label(trap_ref.label)?;
                self.emit(vm::Instr::Arm(env, label))
            },

            ir::Op::Disarm(label) => {
                let label = self.tr_label(label)?;
                self.emit(vm::Instr::Disarm(label))
            },

            ir::Op::Export(id, var) => {
                let id = vm::EnvId(id.0);
                let var = self.tr_var(var)?;
                self.emit(vm::Instr::Blocking(vm::Io::Export(var, id)))
            },

            ir::Op::Let(dst, value) => match value {
                ir::Rvalue::Var(src) => {
                    let dst = self.tr_var(dst)?;
                    let src = self.tr_var(src)?;
                    if src != dst {
                        self.emit(vm::Instr::Cpy(src, dst))
                    } else {
                        Ok(())
                    }
                },

                ir::Rvalue::Int(i) => {
                    let dst = self.tr_var(dst)?;
                    self.emit(vm::Instr::LoadLit(vm::Value::Int(i), dst))
                },

                ir::Rvalue::Const(cr) => {
                    let dst = self.tr_var(dst)?;
                    match cr {
                        ir::ConstRef::Atom(a) => {
                            let lit = self.tr_atom(a)?;
                            self.emit(vm::Instr::LoadLit(lit, dst))
                        },

                        ir::ConstRef::Str(s) => {
                            let lit = self.tr_str(s)?;
                            self.emit(vm::Instr::LoadLit(lit, dst))
                        },
                    }
                },

                ir::Rvalue::Add(lhs, rhs) => {
                    tr_binop(self, vm::Instr::Add, lhs, rhs, dst)
                },

                ir::Rvalue::Sub(lhs, rhs) => {
                    tr_binop(self, vm::Instr::Sub, lhs, rhs, dst)
                },

                ir::Rvalue::Div(lhs, rhs) => {
                    tr_binop(self, vm::Instr::Div, lhs, rhs, dst)
                },

                ir::Rvalue::Mul(lhs, rhs) => {
                    tr_binop(self, vm::Instr::Mul, lhs, rhs, dst)
                },

                ir::Rvalue::Roll(lhs, rhs) => {
                    fn roll(a: vm::Reg, b: vm::Reg) -> vm::Instr {
                        vm::Instr::Blocking(vm::Io::Roll(b, a))
                    }

                    tr_binop(self, roll, lhs, rhs, dst)
                },

                ir::Rvalue::Load(ptr) => {
                    let ptr = vm::Ptr {
                        addr: self.tr_var(ptr.start_addr)?,
                        offset: ptr.offset,
                    };

                    let dst = self.tr_var(dst)?;
                    self.emit(vm::Instr::Read(ptr, dst))
                },

                ir::Rvalue::LoadEnv(offset) => {
                    let ptr = vm::Ptr {
                        addr: vm::Reg::env(),
                        offset: offset,
                    };

                    let dst = self.tr_var(dst)?;
                    self.emit(vm::Instr::Read(ptr, dst))
                },

                ir::Rvalue::FromBool(_) => {
                    ice!("Unimplemented: ir::Rvalue::FromBool")
                },

                ir::Rvalue::Spawn(call) => {
                    let dst = self.tr_var(dst)?;
                    let argv = self.tr_var(call.argv)?;
                    let label = self.tr_label(call.label)?;

                    // FIXME: We need to keep the env table around anyway,
                    // so just look up env IDs dynamically
                    let &env = self.env_table.get(&call.label)
                        .unwrap_or(&vm::EnvId(0xffffffff));

                    self.emit(vm::Instr::Blocking({
                        vm::Io::Spawn(argv, env, label, dst)
                    }))
                },

                ir::Rvalue::Splice(vars) => {
                    ice!("Unimplemented: splice")
                },

                ir::Rvalue::Alloc(size) => {
                    let size = vm::ListLen(size);
                    let dst = self.tr_var(dst)?;
                    self.emit(vm::Instr::Alloc(size, dst))
                },

                ir::Rvalue::PidOfSelf => {
                    let dst = self.tr_var(dst)?;
                    self.emit(vm::Instr::Blocking(vm::Io::GetPid(dst)))
                },

                _ => ice!("Unimplemented: Rvalue {:?}", value),
            },

            ir::Op::Listen(trap_ref) => {
                let env = self.tr_var(trap_ref.env)?;
                let label = self.tr_label(trap_ref.label)?;
                self.emit(vm::Instr::Blocking(vm::Io::ArmAtomic(env, label)))
            },

            ir::Op::Say(var) => {
                let var = self.tr_var(var)?;
                self.emit(vm::Instr::Blocking(vm::Io::Say(var)))
            },

            ir::Op::Store(src, dst) => {
                let dst = vm::Ptr {
                    addr: self.tr_var(dst.start_addr)?,
                    offset: dst.offset,
                };

                let src = self.tr_var(src)?;

                self.emit(vm::Instr::Write(src, dst))
            },

            ir::Op::Wait(val) => {
                // FIXME: Actually translate time units
                self.emit(vm::Instr::Blocking(vm::Io::Sleep(9000.0)))
            },

            _ => ice!("Unimplemented: IR op {:?}", t),
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
                self.emit(vm::Instr::JumpIf(flag, succ))?;
                let fail = self.tr_label(fail)?;
                self.emit(vm::Instr::Jump(fail))
            },

            ir::Exit::Recur(ir::FnCall { argv, label }) => {
                let env_id = match self.env_table.get(&label) {
                    Some(&id) => id,
                    None => ice!("Missing env ID for label"),
                };

                let label = self.tr_label(label)?;
                let argv = self.tr_var(argv)?;
                self.emit(vm::Instr::Blocking({
                    vm::Io::Recur(argv, env_id, label)
                }))
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

    fn tr_atom(&mut self, t: ir::AtomId) -> Try<vm::Value> {
        Ok(vm::Value::Atom(t))
    }

    fn tr_str(&mut self, t: ir::StrId) -> Try<vm::Value> {
        Ok(vm::Value::StrConst(t))
    }

    fn tr_flag(&mut self, t: ir::Flag) -> Try<vm::Flag> {
        Ok(vm::Flag(t.0))
    }

    fn tr_label(&mut self, t: ir::Label) -> Try<vm::Label> {
        Ok(vm::Label(t.0))
    }
}
