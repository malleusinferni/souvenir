use std::collections::HashMap;

use string_interner::StringInterner;

use vecmap::{VecMap, CheckedFrom};

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
            env_table: translator.env_table,
        })
    }

    pub fn build_env_table(&self) -> Try<vm::EnvTable> {
        Ok(self.ep_table.iter().cloned().filter_map(|(label, ep)| {
            if let ir::EntryPoint::Scene { name, argc, env } = ep {
                let label = vm::Label(label.0);
                let env_id = vm::EnvId(env.0);
                Some((label, env_id))
            } else {
                None
            }
        }).collect())
    }
}

struct Translator {
    registers: HashMap<ir::Var, vm::Reg>,
    env_table: vm::EnvTable,
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

                ir::Rvalue::LoadArg(offset) => {
                    let ptr = vm::Ptr {
                        addr: vm::Reg::arg(),
                        offset: offset,
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

                ir::Rvalue::MenuChoice(src) => {
                    let dst = self.tr_var(dst)?;
                    let src = self.tr_var(src)?;
                    self.emit(vm::Instr::Blocking(vm::Io::Ask(src, dst)))
                },

                ir::Rvalue::FromBool(src) => {
                    let dst = self.tr_var(dst)?;
                    let src = self.tr_flag(src)?;
                    self.emit(vm::Instr::Reify(src, dst))
                },

                ir::Rvalue::Spawn(call) => {
                    let dst = self.tr_var(dst)?;
                    let argv = self.tr_var(call.argv)?;
                    let label = self.tr_label(call.label)?;

                    self.emit(vm::Instr::Blocking({
                        vm::Io::Spawn(argv, label, dst)
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
            },

            ir::Op::Set(dst, value) => match value {
                ir::Tvalue::Flag(src) => {
                    let src = self.tr_flag(src)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Set(src, dst))
                },

                ir::Tvalue::HasLen(list, len) => {
                    let list = self.tr_var(list)?;
                    let len = vm::ListLen(len);
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::CheckSize(len, list, dst))
                },

                ir::Tvalue::Eql(lhs, rhs) => {
                    let lhs = self.tr_var(lhs)?;
                    let rhs = self.tr_var(rhs)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Eql(lhs, rhs, dst))
                },

                ir::Tvalue::Gt(lhs, rhs) => {
                    let lhs = self.tr_var(lhs)?;
                    let rhs = self.tr_var(rhs)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Gt(lhs, rhs, dst))
                },

                ir::Tvalue::Gte(lhs, rhs) => {
                    let lhs = self.tr_var(lhs)?;
                    let rhs = self.tr_var(rhs)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Gte(lhs, rhs, dst))
                },

                ir::Tvalue::Lt(lhs, rhs) => {
                    let lhs = self.tr_var(lhs)?;
                    let rhs = self.tr_var(rhs)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Lt(lhs, rhs, dst))
                },

                ir::Tvalue::Lte(lhs, rhs) => {
                    let lhs = self.tr_var(lhs)?;
                    let rhs = self.tr_var(rhs)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Lte(lhs, rhs, dst))
                },

                ir::Tvalue::True => {
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::True(dst))
                },

                ir::Tvalue::False => {
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::False(dst))
                },

                ir::Tvalue::Not(src) => {
                    let src = self.tr_flag(src)?;
                    let dst = self.tr_flag(dst)?;
                    if src != dst {
                        self.emit(vm::Instr::Set(src, dst))?;
                    }
                    self.emit(vm::Instr::Not(dst))
                },

                ir::Tvalue::Nonzero(src) => {
                    let src = self.tr_var(src)?;
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::Nonzero(src, dst))
                },

                ir::Tvalue::And(flags) => {
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::True(dst))?;
                    for flag in flags {
                        let flag = self.tr_flag(flag)?;
                        self.emit(vm::Instr::And(flag, dst))?;
                    }
                    Ok(())
                },

                ir::Tvalue::Or(flags) => {
                    let dst = self.tr_flag(dst)?;
                    self.emit(vm::Instr::True(dst))?;
                    for flag in flags {
                        let flag = self.tr_flag(flag)?;
                        self.emit(vm::Instr::Or(flag, dst))?;
                    }
                    Ok(())
                },
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

            ir::Op::SendMsg(target, message) => {
                let target = self.tr_var(target)?;
                let message = self.tr_var(message)?;
                self.emit(vm::Instr::Blocking({
                    vm::Io::SendMsg(message, target)
                }))
            },

            ir::Op::Store(src, dst) => {
                let dst = vm::Ptr {
                    addr: self.tr_var(dst.start_addr)?,
                    offset: dst.offset,
                };

                let src = self.tr_var(src)?;

                self.emit(vm::Instr::Write(src, dst))
            },

            ir::Op::Trace(var) => {
                let var = self.tr_var(var)?;
                self.emit(vm::Instr::Blocking(vm::Io::Trace(var)))
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
                let label = self.tr_label(label)?;
                let argv = self.tr_var(argv)?;
                self.emit(vm::Instr::Blocking({
                    vm::Io::Recur(argv, label)
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
        // FIXME: Allocate flags
        Ok(vm::Flag(t.0))
    }

    fn tr_label(&mut self, t: ir::Label) -> Try<vm::Label> {
        Ok(vm::Label(t.0))
    }
}
