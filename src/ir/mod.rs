pub mod pass;
pub mod translate;

#[derive(Clone, Debug)]
pub struct Program {
    blocks: Vec<Block>,
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Var(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Flag(pub u32);

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Label(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ConstRef(pub u32);

#[derive(Clone, Debug)]
pub enum ConstValue {
    Atom(String),
    Str(String),
}

#[derive(Clone, Debug)]
pub struct Block {
    pub info: BlockInfo,
    pub ops: Vec<Op>,
    pub exit: Exit,
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub id: u32,
    pub first_reg: u32,
    pub flags_needed: u32,
}

#[derive(Clone, Debug)]
pub struct TrapRef {
    pub label: Label,
    pub env: Var,
}

#[derive(Clone, Debug)]
pub struct FnCall {
    pub label: Label,
    pub argv: Var,
}

#[derive(Clone, Debug)]
pub struct Ptr {
    pub start_addr: Var,
    pub offset: u32,
}

#[derive(Clone, Debug)]
pub enum Op {
    Arm(TrapRef),
    Disarm(Label),
    Discard(Rvalue),
    Let(Var, Rvalue),
    Listen(TrapRef),
    Print(Var),
    Store(Var, Ptr),
    SendMsg(Var, Var),
    Set(Flag, Tvalue),
    Trace(Var),
    Wait(Var),
    Write(Var),
}

#[derive(Clone, Debug)]
pub enum Exit {
    EndProcess,
    Goto(Label),
    IfThenElse(Flag, Label, Label),
    Recur(FnCall),
    Return(bool),
}

#[derive(Clone, Debug)]
pub enum Rvalue {
    Var(Var),
    Int(i32),
    Add(Var, Var),
    Sub(Var, Var),
    Div(Var, Var),
    Mul(Var, Var),
    Roll(Var, Var),
    Load(Ptr),
    FromBool(Flag),
    Spawn(FnCall),
    Splice(Vec<Var>),
    Alloc(u32),
    Const(ConstRef),
    MenuChoice(Var),
    PidOfSelf,
}

#[derive(Clone, Debug)]
pub enum Tvalue {
    Flag(Flag),
    Eql(Var, Var),
    Gt(Var, Var),
    Lt(Var, Var),
    Gte(Var, Var),
    Lte(Var, Var),
    HasLen(Var, Var),
    Nonzero(Var),
    True,
    False,
    And(Vec<Flag>),
    Or(Vec<Flag>),
    Not(Flag),
}

impl Var {
    pub fn at_offset(self, offset: u32) -> Ptr {
        Ptr {
            start_addr: self,
            offset: offset,
        }
    }
}

impl Label {
    pub fn with_argv(self, argv: Var) -> FnCall {
        FnCall {
            label: self,
            argv: argv,
        }
    }
}
