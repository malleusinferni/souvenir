use vecmap::*;

#[derive(Clone, Debug)]
pub struct Program {
    /// String constants in shared memory.
    pub strings: VecMap<StrId, String>,

    /// Instructions from all blocks.
    pub code: VecMap<InstrAddr, Instr>,

    /// Lookup table for the destinations of jump instructions.
    pub jump_table: VecMap<Label, InstrAddr>,
}

/// Unencoded (immediately executable) VM instructions.
///
/// Multi-argument operations follow the convention of `input -> output` in
/// their arguments. So, for example, `Add(a, b)` reads a value from `a` and
/// adds it to `b`.
#[derive(Copy, Clone, Debug)]
pub enum Instr {
    Cpy(Reg, Reg),
    Add(Reg, Reg),
    Sub(Reg, Reg),
    Div(Reg, Reg),
    Mul(Reg, Reg),
    Eql(Reg, Reg, Flag),
    Gte(Reg, Reg, Flag),
    Lte(Reg, Reg, Flag),
    Gt(Reg, Reg, Flag),
    Lt(Reg, Reg, Flag),
    And(Flag, Flag),
    Or(Flag, Flag),
    Not(Flag),
    True(Flag),
    False(Flag),
    LoadLit(Value, Reg),
    Alloc(ListLen, Reg),
    Read(Ptr, Reg),
    Write(Reg, Ptr),
    Jump(Label),
    JumpIf(Flag, Label),
    Spawn(Reg, Label, Reg),
    Recur(Reg, Label),
    Native(Reg, NativeFn, Reg),
    SendMsg(Reg, Reg),
    GetPid(Reg),
    Roll(Reg, Reg),
    Sleep(f32),
    Arm(Reg, Label),
    Disarm(Label),
    Return(bool),
    Nop,
    Bye,
    Hcf,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Atom(u32),
    ActorId(u32),
    StrConst(StrId),
    StrAddr(u32),
    ListAddr(HeapAddr),
    Capacity(u32),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TypeTag {
    Int,
    Atom,
    Actor,
    Str,
    List,
}

#[derive(Copy, Clone, Debug)]
pub struct NativeFn(u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ListLen(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ptr {
    addr: Reg,
    offset: u32,
}

pub type JumpTable = VecMap<Label, InstrAddr>;

pub struct StackFrame {
    gpr: [Value; REG_COUNT],
    flag: [bool; REG_COUNT],
}

/// Prototype for a message handler.
#[derive(Copy, Clone, Debug)]
pub struct Trap {
    label: Label,
    env: HeapAddr,
}

/// State of a handler invocation.
pub struct Continuation {
    /// Code position to return to once there are no handlers left to execute.
    return_addr: InstrAddr,

    /// Input to running handlers.
    message: Value,

    sender: Value,

    frame: StackFrame,

    /// Sequence of remaining handlers in this invocation.
    queue: Vec<Trap>,
}

pub struct Stack {
    lower: StackFrame,
    upper: Option<Continuation>,
}

#[derive(Clone, Debug, Default)]
pub struct Heap {
    values: Vec<Value>,
    strings: Vec<String>,
}

pub struct Process {
    stack: Stack,
    heap: Heap,
    traps: Vec<Trap>,
    op: Instr,
    pc: InstrAddr,
}

#[derive(Copy, Clone, Debug)]
pub enum RunErr {
    Generic,
}

pub type Ret<T> = Result<T, RunErr>;

const REG_COUNT: usize = 0x400;

impl Default for Instr {
    fn default() -> Self { Instr::Nop }
}

impl Default for StackFrame {
    fn default() -> Self {
        StackFrame {
            gpr: [Value::Int(-1); REG_COUNT],
            flag: [false; REG_COUNT],
        }
    }
}

macro_rules! index_via_u32 {
    ( $name:ident, $( $value:ty ),* ) => {
        #[derive(Copy, Clone, Debug, Default, PartialEq)]
        pub struct $name(pub u32);

        impl From<$name> for usize {
            fn from($name(u): $name) -> Self {
                u as usize
            }
        }

        impl CheckedFrom<usize> for $name {
            fn checked_from(u: usize) -> Option<Self> {
                if u > u32::max_value() as usize {
                    None
                } else {
                    Some($name(u as u32))
                }
            }
        }

        $( impl IndexFor<$value> for $name {} )*
    };
}

index_via_u32!(Label, InstrAddr);
index_via_u32!(InstrAddr, Instr);
index_via_u32!(Reg, Value);
index_via_u32!(HeapAddr, Value);
index_via_u32!(Flag, bool);
index_via_u32!(StrId, String);

impl Value {
    pub fn tag(&self) -> TypeTag {
        match self {
            &Value::Int(_) => TypeTag::Int,
            &Value::Atom(_) => TypeTag::Atom,
            &Value::ActorId(_) => TypeTag::Actor,
            &Value::StrConst(_) | &Value::StrAddr(_) => TypeTag::Str,
            &Value::ListAddr(_) | &Value::Capacity(_) => TypeTag::List,
        }
    }
}

impl Stack {
    fn current(&mut self) -> &mut StackFrame {
        if let Some(c) = self.upper.as_mut() {
            return &mut c.frame;
        }

        &mut self.lower
    }

    fn push(&mut self, cc: Continuation) -> Ret<()> {
        if self.upper.is_some() {
            Err(RunErr::Generic)
        } else {
            self.upper = Some(cc);
            Ok(())
        }
    }

    fn pop(&mut self) -> Ret<Continuation> {
        self.upper.take().ok_or(RunErr::Generic)
    }
}

impl StackFrame {
    fn get(&self, r: Reg) -> Ret<Value> {
        let i = r.0 as usize;
        if i < REG_COUNT {
            Ok(self.gpr[i])
        } else {
            Err(RunErr::Generic)
        }
    }

    fn set(&mut self, r: Reg, v: Value) -> Ret<()> {
        let i = r.0 as usize;
        if i < REG_COUNT {
            self.gpr[i] = v;
            Ok(())
        } else {
            Err(RunErr::Generic)
        }
    }

    fn get_flag(&mut self, f: Flag) -> Ret<bool> {
        let i = f.0 as usize;
        if i < REG_COUNT {
            Ok(self.flag[i])
        } else {
            Err(RunErr::Generic)
        }
    }

    fn set_flag(&mut self, f: Flag, v: bool) -> Ret<()> {
        let i = f.0 as usize;
        if i < REG_COUNT {
            self.flag[i] = v;
            Ok(())
        } else {
            Err(RunErr::Generic)
        }
    }
}

impl Heap {
    fn alloc(&mut self, len: ListLen) -> Ret<HeapAddr> {
        let addr = HeapAddr(self.values.len() as u32);
        self.values.push(len.into());
        for _ in 0 .. len.0 {
            self.values.push(Value::Atom(0));
        }
        Ok(addr)
    }

    fn check_bounds(&self, addr: HeapAddr, offset: u32) -> Ret<usize> {
        let addr: usize = addr.into();
        let header = *self.values.get(addr).ok_or(RunErr::Generic)?;
        if let Value::Capacity(c) = header {
            if c > offset {
                Ok(addr + offset as usize)
            } else {
                Err(RunErr::Generic)
            }
        } else {
            Err(RunErr::Generic)
        }
    }

    fn get(&self, addr: HeapAddr, offset: u32) -> Ret<Value> {
        let i = self.check_bounds(addr, offset)?;
        Ok(self.values[i])
    }

    fn set(&mut self, addr: HeapAddr, offset: u32, value: Value) -> Ret<()> {
        let i = self.check_bounds(addr, offset)?;
        self.values[i] = value;
        Ok(())
    }
}

impl Process {
    pub fn exec(&mut self, program: &Program) -> Ret<()> {
        match self.op {
            Instr::Nop => (),

            Instr::Cpy(src, dst) => {
                let frame = self.stack.current();
                let value = frame.get(src)?;
                frame.set(dst, value)?;
            },

            Instr::Add(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs + rhs).into())?;
            },

            Instr::Sub(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs - rhs).into())?;
            },

            Instr::Div(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                if rhs == 0 {
                    return Err(RunErr::Generic);
                } else {
                    frame.set(dst, (lhs / rhs).into())?;
                }
            },

            Instr::Mul(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs * rhs).into())?;
            },

            Instr::Eql(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?;
                let rhs = frame.get(rhs)?;
                frame.set_flag(flag, lhs == rhs)?;
            },

            Instr::Gte(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs >= rhs)?;
            },

            Instr::Lte(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs <= rhs)?;
            },

            Instr::Gt(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs > rhs)?;
            },

            Instr::Lt(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs < rhs)?;
            },

            Instr::And(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get_flag(dst)?;
                let rhs = frame.get_flag(src)?;
                frame.set_flag(dst, lhs && rhs)?;
            },

            Instr::Or(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get_flag(dst)?;
                let rhs = frame.get_flag(src)?;
                frame.set_flag(dst, lhs || rhs)?;
            },

            Instr::Not(flag) => {
                let frame = self.stack.current();
                let value = frame.get_flag(flag)?;
                frame.set_flag(flag, !value)?;
            },

            Instr::True(flag) => {
                self.stack.current().set_flag(flag, true)?;
            },

            Instr::False(flag) => {
                self.stack.current().set_flag(flag, false)?;
            },

            Instr::LoadLit(value, dst) => {
                self.stack.current().set(dst, value)?;
            },

            Instr::Alloc(len, dst) => {
                let addr = self.heap.alloc(len)?;
                self.stack.current().set(dst, addr.into())?;
            },

            Instr::Read(ptr, dst) => {
                let frame = self.stack.current();
                let addr = frame.get(ptr.addr)?.as_addr()?;
                let value = self.heap.get(addr, ptr.offset)?;
                frame.set(dst, value)?;
            },

            Instr::Write(src, ptr) => {
                let frame = self.stack.current();
                let value = frame.get(src)?;
                let addr = frame.get(ptr.addr)?.as_addr()?;
                self.heap.set(addr, ptr.offset, value)?;
            },

            Instr::Jump(label) => {
                self.pc = *program.jump_table.get(label)?;
            },

            Instr::JumpIf(flag, label) => {
                if self.stack.current().get_flag(flag)? {
                    self.pc = *program.jump_table.get(label)?;
                }
            },

            Instr::Return(finished) => {
                let mut cc = self.stack.pop()?;
                self.pc = cc.return_addr;

                if !finished {
                    self.run_handler(cc, program)?;
                }
            },

            Instr::Arm(env, label) => {
                self.traps.retain(|trap| trap.label != label);

                let addr = self.stack.current().get(env)?.as_addr()?;
                self.traps.push(Trap {
                    env: addr,
                    label: label,
                });
            },

            Instr::Disarm(label) => {
                self.traps.retain(|trap| trap.label != label);
            },

            // Can't execute other instructions by ourselves
            _ => return Err(RunErr::Generic),
        }

        Ok(())
    }

    fn run_handler(&mut self, mut cc: Continuation, program: &Program) -> Ret<()> {
        let trap = match cc.queue.pop() {
            Some(trap) => trap,
            None => return Ok(()),
        };

        cc.frame.gpr[0] = trap.env.into();
        cc.frame.gpr[1] = cc.message;
        cc.frame.gpr[2] = cc.sender;

        self.pc = *program.jump_table.get(trap.label)?;

        Ok(())
    }
}

impl Value {
    fn as_int(self) -> Ret<i32> {
        match self {
            Value::Int(i) => Ok(i),
            _ => Err(RunErr::Generic),
        }
    }

    fn as_bool(self) -> Ret<bool> {
        Ok(self.as_int()? != 0)
    }

    fn as_addr(self) -> Ret<HeapAddr> {
        match self {
            Value::ListAddr(addr) => Ok(addr),
            _ => Err(RunErr::Generic),
        }
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i)
    }
}

impl From<ListLen> for Value {
    fn from(len: ListLen) -> Self {
        Value::Capacity(len.0)
    }
}

impl From<HeapAddr> for Value {
    fn from(addr: HeapAddr) -> Self {
        Value::ListAddr(addr)
    }
}

impl From<IndexErr<Label>> for RunErr {
    fn from(err: IndexErr<Label>) -> Self {
        let _ = err; // FIXME
        RunErr::Generic
    }
}
