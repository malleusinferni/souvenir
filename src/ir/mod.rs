pub mod eval;

#[derive(Clone, Debug)]
pub struct Program {
    pub strings: Vec<String>,
    pub code: Vec<Block>,
    pub funcs: Vec<FuncRef>,
    pub modenvs: Vec<eval::Process>,
}

#[derive(Clone, Debug)]
pub struct Block(pub Vec<Instr>);

#[derive(Copy, Clone, Debug)]
pub enum Instr {
    Eval(StackFn),
    Write,
    Enclose,
    Trim(StackAddr),
    PushLit(Value),
    PushVar(StackAddr),
    Jump(BlockID),
    JumpIf(BlockID),
    Spawn(FuncID),
    Recur(FuncID),
    SendMessage,
    Sleep(f32),
    TrapInstall(BlockID),
    TrapEnable(u32),
    TrapDisable(u32),
    TrapReject,
    TrapResume,
    Nop,
    Bye,
    Hcf,
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    Undefined,
    Int(i32),
    Atom(u32),
    ActorId(u32),
    ConstStrId(u32),
    HeapStrAddr(u32),
    HeapListAddr(u32),
}

#[derive(Copy, Clone, Debug)]
pub enum StackFn {
    Add,
    Sub,
    Div,
    Mul,
    Not,
    Swap,
    Roll,
    GetPid,
    Discard,
    Native(u32),
}

#[derive(Copy, Clone, Debug)]
pub struct StackAddr(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct BlockID(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct FuncID(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct ModuleID(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct FuncRef {
    module: ModuleID,
    block: BlockID,
}

#[cfg(test)]
pub mod example {
    use ir::*;

    pub static ADD_TWO_NUMBERS: &'static [Instr] = &[
        Instr::PushLit(Value::Int(2)),
        Instr::PushLit(Value::Int(2)),
        Instr::Eval(StackFn::Add),
        Instr::Write,
    ];
}
