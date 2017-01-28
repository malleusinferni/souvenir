use std::collections::VecDeque;

use ir::*;

#[derive(Clone, Debug)]
pub struct Process {
    /// Variables in current lexical scope.
    pub stack: Vec<Value>,

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

    /// Write barrier: Index of first writable value on the stack.
    pub wb: u32,

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
    pub stack: Vec<Value>,
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

pub enum AddrSpace {
    Stack,
    Heap,
    StringHeap,
}

pub enum RunErr {
    SegfaultIn(AddrSpace),
    CorruptionIn(AddrSpace),
    IllegalInstr(Instr),
}

pub type Ret<T> = Result<T, RunErr>;

impl Process {
    pub fn step(&mut self) -> Ret<()> {
        match self.op {
            Instr::TrapInstall(st) => {
                self.traps.push_front(Trap {
                    st: st,
                    wb: self.wb,
                    armed: true,
                });
            },

            Instr::PushVar(StackAddr(u)) => {
                let val = self.stack_load(u as usize)?;
                self.stack_push(val)?;
            },

            _ => unimplemented!(),
        }

        Ok(())
    }

    pub fn copy_args(&mut self, other: &Process) -> Ret<()> {
        for &arg in other.working_set()? {
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

            self.stack.push(arg);
            self.wb += 1;
        }

        Ok(())
    }

    pub fn working_set(&self) -> Ret<&[Value]> {
        unimplemented!()
    }

    pub fn stack_load(&self, addr: usize) -> Ret<Value> {
        let active_stack = match &self.ts {
            &Some(ref ts) => &ts.stack,
            &None => &self.stack,
        };

        // FIXME: Better error detection
        Ok(active_stack[addr])
    }

    pub fn stack_push(&mut self, value: Value) -> Ret<()> {
        if let Some(ts) = self.ts.as_mut() {
            ts.stack.push(value);
        } else {
            self.stack.push(value);
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
