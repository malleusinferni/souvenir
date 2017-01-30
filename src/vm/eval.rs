use vm::*;

#[derive(Clone, Debug, Default)]
pub struct Process {
    /// Variables in current lexical scope.
    pub stack: Stack,

    /// Data structures referenced by live variables.
    pub heap: Heap,

    /// Strings generated at runtime.
    pub strings: Streap,

    /// Available message handlers.
    pub traps: Vec<Trap>,

    /// Prefetched instruction. Signifies current process state.
    pub op: Instr,

    /// Next instruction to be executed.
    pub pc: Address,
}

#[derive(Copy, Clone, Debug)]
pub struct Trap {
    /// Entry point of trap.
    pub label: Label,

    /// Heap address of the trap's closure environment.
    pub env: Value,
}

#[derive(Clone, Debug)]
pub struct Stack {
    /// Values held in the stack.
    pub contents: Vec<Value>,

    /// Starting index of the working set.
    pub wb: u32,

    /// Message handler stack frame, if a message handler is running.
    pub ts: Option<TrapState>,
}

#[derive(Copy, Clone, Debug)]
pub struct TrapState {
    /// Return address to restore execution when the last handler exits.
    pub ra: Address,

    /// Index on the heap of the message to handle.
    pub arg: Value,

    /// Index into `traps` of the currently executing handler.
    pub id: u32,

    /// Offset into the parent stack where the local frame begins.
    pub sp: u32,

    /// Starting index of the working set.
    pub wb: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Heap {
    pub contents: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct Streap {
    pub contents: Vec<String>,
}

#[derive(Copy, Clone, Debug)]
pub enum AddrSpace {
    Stack,
    Heap,
    StringHeap,
}

#[derive(Copy, Clone, Debug)]
pub enum TypeTag {
    List,
    Str,
    Atom,
    ActorId,
    Integer,
}

#[derive(Copy, Clone, Debug)]
pub enum RunErr {
    SegfaultIn(AddrSpace),
    CorruptionIn(AddrSpace),
    StackOverflow,
    StackUnderflow,
    NoSuchLabel(Label),
    IllegalInstr(Instr),
    BadFetch(Address),
    WrongType(Value, TypeTag),
    DividedByZero,
}

pub type Ret<T> = Result<T, RunErr>;

impl Process {
    pub fn exec(&mut self, jump_table: &[Address]) -> Ret<()> {
        match self.op {
            Instr::Nop => (),

            Instr::Jump(label) => {
                self.jump(label, jump_table)?;
            },

            Instr::JumpIf(label) => {
                if self.stack.pop()?.as_bool()? {
                    self.jump(label, jump_table)?;
                }
            },

            Instr::TrapInstall(label) => {
                let env = self.heap.write(self.stack.read_registers())?;

                self.traps.push(Trap {
                    label: label,
                    env: env,
                });
            },

            Instr::TrapRemove(label) => {
                for trap in self.traps.iter_mut().rev() {
                    if trap.label != label { continue; }
                    trap.disarm();
                }
            },

            Instr::TrapReject => {
                let ts: TrapState = self.stack.leave()?;
                self.pc = ts.ra;

                if let Some(next) = ts.id.checked_sub(1) {
                    let &Trap { label, env } = self.traps.get(next as usize)
                        .ok_or(RunErr::IllegalInstr(self.op))?;

                    {
                        let env = self.heap.read(env.as_list_addr()?)?;
                        self.stack.enter(self.pc, next, ts.arg, env)?;
                    }

                    self.stack.push(ts.arg)?;
                    self.stack.write()?;

                    self.jump(label, jump_table)?;
                }
            },

            Instr::TrapResume => {
                let ts: TrapState = self.stack.leave()?;
                self.pc = ts.ra;
            },

            Instr::PushVar(StackAddr(u)) => {
                let val = self.stack.read(u as usize)?;
                self.stack.push(val)?;
            },

            Instr::PushLit(val) => {
                self.stack.push(val)?;
            },

            Instr::Eval(f) => {
                self.stack.eval(f)?;
            },

            Instr::Write => {
                self.stack.write()?;
            },

            Instr::Trim(StackAddr(u)) => {
                self.stack.trim(u as usize)?;
            },

            Instr::Enclose => {
                let contents = self.stack.take_working_set();
                let addr = self.heap.write(&contents)?;
                self.stack.push(addr)?;
            },

            _ => unimplemented!(),
        }

        Ok(())
    }

    pub fn jump(&mut self, label: Label, jump_table: &[Address]) -> Ret<()> {
        let Label(u) = label;
        let &addr = jump_table.get(u as usize)
            .ok_or(RunErr::NoSuchLabel(label))?;
        self.pc = addr;

        Ok(())
    }

    pub fn fetch(&mut self, code: &[Instr]) -> Ret<()> {
        let Address(i) = self.pc;

        self.op = *code.get(i as usize)
            .ok_or(RunErr::BadFetch(self.pc))?;

        Ok(())
    }

    pub fn get_fn_args(&mut self, other: &Process) -> Ret<usize> {
        let argv = other.stack.read_working_set();
        let argc = argv.len();

        for &value in argv {
            let local = self.local_copy(value, other)?;
            self.stack.push(local)?;
            self.stack.write()?;
        }

        Ok(argc)
    }

    pub fn local_copy(&mut self, v: Value, other: &Process) -> Ret<Value> {
        match v {
            Value::HeapListAddr(u) => {
                let list = other.heap.read(u as usize)?;
                let mut buf = Vec::with_capacity(list.len());
                for &value in list {
                    buf.push(self.local_copy(value, other)?);
                }
                self.heap.write(&buf)
            },

            Value::HeapStrAddr(u) => {
                let string = other.strings.read(u as usize)?;
                self.strings.write(string.to_owned())
            },

            other => Ok(other),
        }
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack {
            contents: Vec::with_capacity(32),
            wb: 0,
            ts: None,
        }
    }
}

impl Stack {
    fn write_barrier(&self) -> usize {
        match self.ts {
            Some(ts) => ts.wb as usize,
            None => self.wb as usize,
        }
    }

    pub fn push(&mut self, value: Value) -> Ret<()> {
        self.contents.push(value);

        Ok(())
    }

    pub fn pop(&mut self) -> Ret<Value> {
        if self.contents.len() > self.write_barrier() {
            Ok(self.contents.pop().unwrap())
        } else {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        }
    }

    pub fn read(&self, mut addr: usize) -> Ret<Value> {
        if let Some(ts) = self.ts {
            addr += ts.sp as usize;
        }

        if addr < self.write_barrier() {
            Ok(self.contents[addr])
        } else {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        }
    }

    pub fn write(&mut self) -> Ret<()> {
        if let Some(ts) = self.ts.as_mut() {
            ts.wb += 1;
        } else {
            self.wb += 1;
        }

        // NOTE: These can be equal, meaning the working set is empty
        if self.write_barrier() > self.contents.len() {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        } else {
            Ok(())
        }
    }

    pub fn trim(&mut self, mut len: usize) -> Ret<()> {
        if let Some(ts) = self.ts {
            len += ts.sp as usize;
        }

        self.contents.truncate(len);

        if let Some(ts) = self.ts.as_mut() {
            ts.wb = len as u32;
        } else {
            self.wb = len as u32;
        }

        Ok(())
    }

    pub fn enter(&mut self, ra: Address, id: u32, arg: Value, env: &[Value]) -> Ret<()> {
        if self.ts.is_some() { return Err(RunErr::StackOverflow); }

        let sp = self.contents.len() as u32;

        self.ts = Some(TrapState {
            ra: ra,
            id: id,
            arg: arg,
            sp: sp,
            wb: sp,
        });

        for &value in env {
            self.push(value)?;
            self.write()?;
        }

        Ok(())
    }

    pub fn leave(&mut self) -> Ret<TrapState> {
        self.ts.take().ok_or(RunErr::StackUnderflow)
    }

    pub fn read_working_set(&self) -> &[Value] {
        &self.contents[self.write_barrier() ..]
    }

    pub fn take_working_set(&mut self) -> Vec<Value> {
        let wb = self.write_barrier();
        self.contents.split_off(wb)
    }

    pub fn read_registers(&self) -> &[Value] {
        match self.ts {
            Some(ts) => &self.contents[ts.sp as usize .. ts.wb as usize],
            None => &self.contents[0 .. self.wb as usize],
        }
    }

    pub fn eval(&mut self, f: StackFn) -> Ret<()> {
        match f {
            StackFn::Add => {
                let rhs = self.pop()?.as_int()?;
                let lhs = self.pop()?.as_int()?;
                self.push(Value::Int(lhs + rhs))
            },

            StackFn::Sub => {
                let rhs = self.pop()?.as_int()?;
                let lhs = self.pop()?.as_int()?;
                self.push(Value::Int(lhs - rhs))
            },

            StackFn::Div => {
                let rhs = self.pop()?.as_int()?;
                if rhs == 0 { return Err(RunErr::DividedByZero); }
                let lhs = self.pop()?.as_int()?;
                self.push(Value::Int(lhs / rhs))
            },

            StackFn::Mul => {
                let rhs = self.pop()?.as_int()?;
                let lhs = self.pop()?.as_int()?;
                self.push(Value::Int(lhs * rhs))
            },

            StackFn::Not => {
                let val = match self.pop()?.as_bool()? {
                    true => 0,
                    false => 1,
                };
                self.push(Value::Int(val))
            },

            StackFn::Swap => {
                let rhs = self.pop()?;
                let lhs = self.pop()?;
                self.push(rhs)?;
                self.push(lhs)
            },

            StackFn::Discard => {
                self.pop()?;
                Ok(())
            },
        }
    }
}

impl Heap {
    pub fn write(&mut self, values: &[Value]) -> Ret<Value> {
        let addr = self.contents.len() as u32;
        self.contents.push(Value::Int(values.len() as i32));
        self.contents.extend_from_slice(values);
        Ok(Value::HeapListAddr(addr))
    }

    pub fn read(&self, addr: usize) -> Ret<&[Value]> {
        let &header = self.contents.get(addr)
            .ok_or(RunErr::SegfaultIn(AddrSpace::Heap))?;

        let start = addr + 1;

        let length = match header {
            Value::Int(n) if n >= 0 => Ok(n as usize),
            _ => Err(RunErr::CorruptionIn(AddrSpace::Heap)),
        }?;

        if start + length > self.contents.len() {
            Err(RunErr::SegfaultIn(AddrSpace::Heap))
        } else {
            Ok(&self.contents[start .. start + length])
        }
    }
}

impl Streap {
    pub fn write(&mut self, s: String) -> Ret<Value> {
        let addr = self.contents.len() as u32;
        self.contents.push(s);
        Ok(Value::HeapStrAddr(addr))
    }

    pub fn read(&self, addr: usize) -> Ret<&str> {
        self.contents.get(addr)
            .map(|s| s.as_ref())
            .ok_or(RunErr::SegfaultIn(AddrSpace::StringHeap))
    }
}

impl Value {
    pub fn as_bool(self) -> Ret<bool> {
        match self {
            Value::Int(0) => Ok(false),
            Value::Int(_) => Ok(true),
            // TODO: Consider truthiness of other values
            other => Err(RunErr::WrongType(other, TypeTag::Integer)),
        }
    }

    pub fn as_int(self) -> Ret<i32> {
        match self {
            Value::Int(i) => Ok(i),
            other => Err(RunErr::WrongType(other, TypeTag::Integer)),
        }
    }

    pub fn as_list_addr(self) -> Ret<usize> {
        match self {
            Value::HeapListAddr(n) => Ok(n as usize),
            other => Err(RunErr::WrongType(other, TypeTag::List)),
        }
    }
}

impl Trap {
    fn disarm(&mut self) {
        unimplemented!()
    }
}
