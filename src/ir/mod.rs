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
    use ir::eval::Process;

    impl Process {
        fn evaluate(&mut self, code: &[Instr]) {
            for &instr in code {
                self.op = instr;
                self.exec().unwrap();
            }
        }
    }

    static ADD_TWO_NUMBERS: &'static [Instr] = &[
        Instr::PushLit(Value::Int(2)),
        Instr::PushLit(Value::Int(2)),
        Instr::Eval(StackFn::Add),
        Instr::Write,
    ];

    #[test]
    fn two_plus_two() {
        let mut p = Process::default();
        p.evaluate(ADD_TWO_NUMBERS);
        assert_eq!(&p.stack.contents, &[Value::Int(4)]);
    }

    static BUILD_A_LIST: &'static [Instr] = &[
        Instr::PushLit(Value::Int(1)),
        Instr::PushLit(Value::Int(2)),
        Instr::PushLit(Value::Int(3)),
        Instr::PushLit(Value::Int(4)),
        Instr::Enclose,
        Instr::Write,
    ];

    #[test]
    fn build_a_list() {
        let mut p = Process::default();
        p.evaluate(BUILD_A_LIST);
        assert_eq!(&p.stack.contents, &[Value::HeapListAddr(0)]);
    }
}
