pub mod eval;

#[derive(Clone, Debug)]
pub struct Program {
    /// String constants in shared memory.
    pub strings: Vec<String>,

    /// Instructions from all blocks.
    pub code: Vec<Instr>,

    /// Maps `Label`s to indices into `code`.
    pub labels: Vec<Address>,

    /// Table of function definitions. Indexed by `FuncID`.
    pub funcs: Vec<FuncDef>,

    /// Pre-evaluated environments for module-scoped variables.
    pub modenvs: Vec<eval::Process>,
}

#[derive(Copy, Clone, Debug)]
pub struct Address(pub u32);

#[derive(Copy, Clone, Debug)]
pub enum Instr {
    Eval(StackFn),
    Write,
    Enclose,
    Trim(StackAddr),
    PushLit(Value),
    PushVar(StackAddr),
    Jump(Label),
    JumpIf(Label),
    Spawn(FuncID),
    Recur(FuncID),
    Native(NativeFn),
    SendMessage(StackAddr),
    Sleep(f32),
    TrapInstall(Label),
    TrapRemove(Label),
    TrapReject,
    TrapResume,
    Nop,
    Bye,
    Hcf,
}

#[derive(Copy, Clone, Debug, PartialEq)]
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
    Discard,
}

#[derive(Copy, Clone, Debug)]
pub enum NativeFn {
    Roll,
    GetPid,
    Custom(u32),
}

#[derive(Copy, Clone, Debug)]
pub struct StackAddr(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Label(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct FuncID(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct ModuleID(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct FuncDef {
    module: ModuleID,
    block: Label,
}

impl Default for Instr {
    fn default() -> Self { Instr::Nop }
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

    #[test]
    fn two_plus_two() {
        use ir::eval::Process;

        let mut p = Process::default();

        for &instr in ADD_TWO_NUMBERS {
            p.op = instr;
            p.exec().unwrap();
        }

        assert_eq!(&p.stack.contents, &[Value::Int(4)]);
    }
}
