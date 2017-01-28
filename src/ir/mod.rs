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
    Freeze(StackAddr),
    Trim(StackAddr),
    PushLit(Value),
    PushVar(StackAddr),
    PushMyPid,
    Jump(BlockID),
    JumpIf(BlockID),
    Spawn(FuncID),
    Recur(FuncID),
    SendMessage,
    Nop,
    Bye,
    Hcf,
}

#[derive(Copy, Clone, Debug)]
pub enum Value {
    Undefined,
    Int(u32),
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
    Roll,
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
