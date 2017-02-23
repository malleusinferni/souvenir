use std::fmt::{self, Display};

use vm::*;

impl Display for RawValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &RawValue::ActorId(ActorId(id)) => write!(f, "?PID({})", id),

            &RawValue::Int(i) => write!(f, "{}", i),

            &RawValue::Str(ref s) => write!(f, "> {}", s),

            &RawValue::Atom(ref a) => write!(f, "#{}", a),

            &RawValue::List(ref values) => {
                write!(f, "[{}]", values.iter().map(|value| {
                    format!("{}", value)
                }).collect::<Vec<_>>().join(", "))
            },
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (name, def) in self.scene_table.iter() {
            writeln!(f, "scene {}/{}:\t{}", name, def.argc, def.label)?;
        }

        let mut pairs: Vec<_> = self.jump_table.iter()
            .map(|(k, v)| (v.clone(), k))
            .collect();

        pairs.sort_by_key(|&(InstrAddr(addr), _)| addr);
        pairs.reverse();

        let mut label = pairs.pop();

        for (line, instr) in self.code.iter() {
            match label {
                Some((addr, name)) if line == addr => {
                    writeln!(f, "{}:", name)?;
                    label = pairs.pop();
                },

                _ => (),
            }

            writeln!(f, "\t{}", instr)?;
        }

        for (key, value) in self.str_table.iter() {
            writeln!(f, "STR({}):", key.0)?;
            writeln!(f, "\t{:?}", value)?;
        }

        Ok(())
    }
}

impl Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Instr::Bye => write!(f, "bye"),
            &Instr::Hcf => write!(f, "hcf"),
            &Instr::Nop => write!(f, "nop"),

            &Instr::Return(result) => write!(f, "ret {}", result),
            &Instr::Arm(reg, label) => write!(f, "arm {}, {}", reg, label),
            &Instr::Disarm(label) => write!(f, "dis {}", label),

            &Instr::LoadLit(lit, dst) => write!(f, "let {} -> {}", lit, dst),
            &Instr::Cpy(src, dst) => write!(f, "let {} -> {}", src, dst),
            &Instr::Read(src, dst) => write!(f, "let {} -> {}", src, dst),
            &Instr::Write(src, dst) => write!(f, "let {} -> {}", src, dst),

            &Instr::Add(src, dst) => write!(f, "add {} -> {}", src, dst),
            &Instr::Sub(src, dst) => write!(f, "sub {} -> {}", src, dst),
            &Instr::Div(src, dst) => write!(f, "div {} -> {}", src, dst),
            &Instr::Mul(src, dst) => write!(f, "mul {} -> {}", src, dst),

            &Instr::Set(src, dst) => write!(f, "test {} -> {}", src, dst),

            &Instr::Not(dst) => write!(f, "not {}", dst),

            &Instr::Nonzero(src, dst) => {
                write!(f, "test nonzero {} -> {}", src, dst)
            },

            &Instr::Eql(lhs, rhs, dst) => {
                write!(f, "test {} eq {} -> {}", lhs, rhs, dst)
            },

            &Instr::Gt(lhs, rhs, dst) => {
                write!(f, "test {} gt {} -> {}", lhs, rhs, dst)
            },

            &Instr::Gte(lhs, rhs, dst) => {
                write!(f, "test {} gte {} -> {}", lhs, rhs, dst)
            },

            &Instr::Lt(lhs, rhs, dst) => {
                write!(f, "test {} lt {} -> {}", lhs, rhs, dst)
            },

            &Instr::Lte(lhs, rhs, dst) => {
                write!(f, "test {} lte {} -> {}", lhs, rhs, dst)
            },

            &Instr::CheckSize(ListLen(len), src, dst) => {
                write!(f, "test len({}) eq {} -> {}", src, len, dst)
            },

            &Instr::Reify(src, dst) => {
                write!(f, "let int({}) -> {}", src, dst)
            },

            &Instr::And(src, dst) => write!(f, "and {} -> {}", src, dst),

            &Instr::Or(src, dst) => write!(f, "or {} -> {}", src, dst),

            &Instr::True(dst) => write!(f, "true -> {}", dst),

            &Instr::False(dst) => write!(f, "false -> {}", dst),

            &Instr::Alloc(size, dst) => write!(f, "alloc {} -> {}", size, dst),

            &Instr::Jump(label) => write!(f, "jump {}", label),

            &Instr::JumpIf(flag, label) => {
                write!(f, "if {} jump {}", flag, label)
            },

            &Instr::Blocking(io) => match io {
                Io::Trace(src) => write!(f, "trace {}", src),

                Io::Export(src, EnvId(id)) => {
                    write!(f, "export {} -> {}", src, id)
                },

                Io::Spawn(arg, label, dst) => {
                    write!(f, "spawn {}, {} -> {}", arg, label, dst)
                },

                Io::Recur(arg, label) => {
                    write!(f, "recur {}, {}", arg, label)
                },

                Io::GetPid(dst) => {
                    write!(f, "self -> {}", dst)
                },

                Io::SendMsg(msg, dst) => {
                    write!(f, "send {} -> {}", msg, dst)
                },

                Io::Roll(src, dst) => {
                    write!(f, "roll {} -> {}", src, dst)
                },

                Io::Sleep(amt) => {
                    write!(f, "sleep {}", amt)
                },

                Io::Say(src) => {
                    write!(f, "say {}", src)
                },

                Io::Native(arg, NativeFn(func), dst) => {
                    write!(f, "syscall {}, {} -> {}", func, arg, dst)
                },

                Io::ArmAtomic(env, label) => {
                    write!(f, "listen {}, {}", env, label)
                },

                Io::Ask(src, dst) => {
                    write!(f, "ask {} -> {}", src, dst)
                },
            },
        }
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &Reg(id) = self;
        write!(f, "%r{:x}", id)
    }
}

impl Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &Flag(id) = self;
        write!(f, "?{:X}", id)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Int(i) => write!(f, "{}i", i),
            &Value::Atom(AtomId(a)) => write!(f, "#{}", a),
            &Value::ActorId(ActorId(a)) => write!(f, "&PID({})", a),
            &Value::StrConst(StrId(s)) => write!(f, "&STR({})", s),
            &Value::StrAddr(s) => write!(f, "&DYN({})", s),
            &Value::ListAddr(HeapAddr(h)) => write!(f, ".{:X}", h),
            &Value::Capacity(c) => write!(f, "0x{:X}", c),
            &Value::Undefined => write!(f, "UNDEF"),
        }
    }
}

impl Display for ListLen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &ListLen(len) = self;
        write!(f, "{}", len)
    }
}

impl Display for Ptr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} @ {}", self.addr, self.offset)
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{:X}", self.0)
    }
}
