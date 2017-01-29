use std::collections::VecDeque;

use ir::*;

#[derive(Clone, Debug)]
pub struct Process {
    /// Variables in current lexical scope.
    pub stack: Stack,

    /// Data structures referenced by live variables.
    pub heap: Vec<Value>,

    /// Strings generated at runtime.
    pub strings: Vec<String>,

    /// Available message handlers.
    pub traps: VecDeque<Trap>,

    /// Prefetched instruction. Signifies current process state.
    pub op: Instr,

    /// Block and offset of next instruction to be executed.
    pub pc: (BlockID, u32),

    /// State of the current message handler, if one is active.
    pub ts: Option<TrapState>,
}

#[derive(Clone, Debug)]
pub struct TrapState {
    /// Index into the process's list of available handlers.
    pub id: u32,

    /// Block and offset of next instruction to be executed.
    pub pc: (BlockID, u32),

    /// Local copy of the stack.
    pub stack: Stack,
}

#[derive(Clone, Debug)]
pub struct Trap {
    /// Entry point of trap.
    pub st: BlockID,

    /// Whether this trap should be ignored when delivering messages.
    pub armed: bool,

    /// Starting index of the working set.
    pub wb: u32,
}

#[derive(Clone, Debug)]
pub struct Stack {
    pub contents: Vec<Value>,

    /// Starting index of the working set.
    pub wb: u32,
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
    IllegalInstr(Instr),
    BadFetch(BlockID, u32),
    WrongType(Value, TypeTag),
    DividedByZero,
}

pub type Ret<T> = Result<T, RunErr>;

impl Process {
    pub fn new() -> Self {
        Process {
            stack: Stack {
                contents: vec![],
                wb: 0,
            },
            heap: vec![],
            strings: vec![],
            traps: VecDeque::new(),
            ts: None,
            op: Instr::Nop,
            pc: (BlockID(0), 0),
        }
    }

    pub fn step(&mut self) -> Ret<()> {
        match self.op {
            Instr::TrapInstall(st) => {
                if self.ts.is_some() {
                    return Err(RunErr::IllegalInstr(self.op));
                }

                self.traps.push_front(Trap {
                    st: st,
                    wb: self.stack.wb,
                    armed: true,
                });
            },

            Instr::TrapEnable(u) => {
                match self.traps.get_mut(u as usize) {
                    Some(trap) => trap.armed = true,
                    None => return Err(RunErr::IllegalInstr(self.op)),
                }
            },

            Instr::TrapDisable(u) => {
                match self.traps.get_mut(u as usize) {
                    Some(trap) => trap.armed = false,
                    None => return Err(RunErr::IllegalInstr(self.op)),
                }
            },

            Instr::PushVar(StackAddr(u)) => {
                let val = self.stack_ref().read(u as usize)?;
                self.stack_mut().push(val)?;
            },

            Instr::PushLit(val) => {
                self.stack_mut().push(val)?;
            },

            Instr::Eval(f) => {
                self.stack_mut().eval(f)?;
            },

            Instr::Write => {
                self.stack_mut().write()?;
            },

            Instr::Trim(StackAddr(u)) => {
                self.stack_mut().trim(u as usize)?;
            },

            Instr::Enclose => {
                unimplemented!()
            },

            _ => unimplemented!(),
        }

        Ok(())
    }

    pub fn fetch(&mut self, program: &Program) -> Ret<()> {
        let (block_id, offset) = match self.ts.as_ref() {
            Some(ts) => ts.pc,
            None => self.pc,
        };

        self.op = *program.code.get(block_id.0 as usize)
            .and_then(|&Block(ref b)| b.get(offset as usize))
            .ok_or(RunErr::BadFetch(block_id, offset))?;

        Ok(())
    }

    pub fn stack_ref(&self) -> &Stack {
        match self.ts.as_ref() {
            Some(ts) => &ts.stack,
            None => &self.stack,
        }
    }

    pub fn stack_mut(&mut self) -> &mut Stack {
        match self.ts.as_mut() {
            Some(ts) => &mut ts.stack,
            None => &mut self.stack,
        }
    }

    pub fn copy_args(&mut self, other: &Process) -> Ret<()> {
        for &arg in other.stack_ref().read_working_set() {
            let mut arg = arg;

            match &mut arg {
                &mut Value::HeapListAddr(ref mut u) => {
                    *u = self.list_copy(*u as usize, other)?;
                },

                &mut Value::HeapStrAddr(ref mut u) => {
                    *u = self.string_write({
                        other.string_read(*u as usize)?.to_owned()
                    })?;
                },

                _ => (),
            }

            self.stack_mut().push(arg)?;
        }

        Ok(())
    }

    pub fn list_copy(&mut self, addr: usize, other: &Process) -> Ret<u32> {
        let list = other.list_read(addr)?;
        let mut buf = Vec::with_capacity(list.len());
        for &value in list {
            buf.push(match value {
                Value::HeapListAddr(u) => {
                    let local_addr = self.list_copy(u as usize, other)?;
                    Value::HeapListAddr(local_addr)
                },

                Value::HeapStrAddr(u) => {
                    let string = other.string_read(u as usize)?.to_owned();
                    Value::HeapStrAddr(self.string_write(string)?)
                },

                // These are stored in global program memory
                //Value::ConstStrId(_) => ...

                other => other,
            })
        }

        self.list_write(buf)
    }

    pub fn list_read(&self, addr: usize) -> Ret<&[Value]> {
        let length = match self.heap.get(addr) {
            Some(&Value::Int(count)) if count >= 0 => count as usize,
            _ => return Err(RunErr::CorruptionIn(AddrSpace::Heap)),
        };

        let start = addr + 1;

        if start + length > self.heap.len() {
            return Err(RunErr::SegfaultIn(AddrSpace::Heap));
        }

        Ok(&self.heap[start .. start + length])
    }

    pub fn list_write(&mut self, values: Vec<Value>) -> Ret<u32> {
        let addr = self.heap.len() as u32;
        self.heap.push(Value::Int(values.len() as i32));
        self.heap.extend(values.into_iter());
        Ok(addr)
    }

    pub fn string_read(&self, addr: usize) -> Ret<&str> {
        self.strings.get(addr)
            .map(|s| s.as_ref())
            .ok_or(RunErr::SegfaultIn(AddrSpace::StringHeap))
    }

    pub fn string_write(&mut self, s: String) -> Ret<u32> {
        let addr = self.strings.len() as u32;
        self.strings.push(s);
        Ok(addr)
    }
}

impl Stack {
    pub fn push(&mut self, value: Value) -> Ret<()> {
        self.contents.push(value);

        Ok(())
    }

    pub fn pop(&mut self) -> Ret<Value> {
        if self.contents.len() > self.wb as usize {
            Ok(self.contents.pop().unwrap())
        } else {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        }
    }

    pub fn read(&self, addr: usize) -> Ret<Value> {
        if addr < self.wb as usize {
            Ok(self.contents[addr])
        } else {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        }
    }

    pub fn write(&mut self) -> Ret<()> {
        self.wb += 1;

        if self.wb as usize > self.contents.len() {
            Err(RunErr::SegfaultIn(AddrSpace::Stack))
        } else {
            Ok(())
        }
    }

    pub fn trim(&mut self, len: usize) -> Ret<()> {
        self.contents.truncate(len);
        self.wb = len as u32;
        Ok(())
    }

    pub fn read_working_set(&self) -> &[Value] {
        &self.contents[self.wb as usize ..]
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
                let val = match self.pop()?.as_int()? {
                    0 => 1,
                    _ => 0,
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

impl Value {
    pub fn as_int(self) -> Ret<i32> {
        match self {
            Value::Int(i) => Ok(i),
            other => Err(RunErr::WrongType(other, TypeTag::Integer)),
        }
    }
}
