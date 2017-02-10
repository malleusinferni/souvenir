#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Var(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Flag(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Label(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ConstId(pub u32);

#[derive(Clone, Debug)]
pub enum ConstValue {
    Atom(String),
    Int(i32),
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
pub enum Op {
    Arm(Label),
    Disarm(Label),
    Discard(Rvalue),
    Let(Var, Rvalue),
    Listen(Label),
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
    Recur(Label, Vec<Var>),
    Return(Tvalue),
}

#[derive(Clone, Debug)]
pub enum Rvalue {
    Var(Var),
    Add(Var, Var),
    Sub(Var, Var),
    Div(Var, Var),
    Mul(Var, Var),
    Roll(Var, Var),
    Nth(Var, Var),
    FromBool(Flag),
    Spawn(Label, Vec<Var>),
    Splice(Vec<Var>),
    ListOf(Vec<Var>),
    Const(ConstId),
    ChoiceFromMenu(Var),
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
