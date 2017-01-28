use ir::*;

#[derive(Clone, Debug)]
pub struct Process {
    pub stack: Vec<Value>,
    pub heap: Vec<Value>,
    pub strings: Vec<String>,

    /// Prefetched instruction. Signifies current process state.
    pub op: Instr,

    /// Block and offset of next instruction to be executed.
    pub pc: (BlockID, u32),

    /// Write barrier: Index of first writable value on the stack.
    pub wb: u32,
}

impl Process {
    pub fn step(&mut self) {
        match self.op {
            _ => unimplemented!(),
        }
    }
}
