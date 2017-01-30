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

#[derive(Copy, Clone, Debug, Default)]
pub struct Address(pub u32);

#[derive(Copy, Clone, Debug)]
pub enum Instr {
    Eval(StackFn),
    Write,
    Enclose,
    Compare(Relation),
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
pub enum Relation {
    Greater,
    Lesser,
    Equal,
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
    use vm::*;
    use vm::eval::Process;

    impl Process {
        fn evaluate(code: &[Instr]) -> Self {
            let mut this = Self::default();

            for &instr in code {
                this.op = instr;
                this.exec(&[]).unwrap();
            }

            this
        }
    }

    #[test]
    fn two_plus_two() {
        let p = Process::evaluate({&[
            Instr::PushLit(Value::Int(2)),
            Instr::PushLit(Value::Int(2)),
            Instr::Eval(StackFn::Add),
            Instr::Write,
        ]});

        assert_eq!(&p.stack.contents, &[Value::Int(4)]);
    }

    #[test]
    fn build_a_list() {
        let p = Process::evaluate({&[
            Instr::PushLit(Value::Int(1)),
            Instr::PushLit(Value::Int(2)),
            Instr::PushLit(Value::Int(3)),
            Instr::PushLit(Value::Int(4)),
            Instr::Enclose,
            Instr::Write,
        ]});

        assert_eq!(&p.stack.contents, &[Value::HeapListAddr(0)]);
    }

    #[test]
    #[should_panic]
    fn stack_underflow() {
        Process::evaluate({&[
            Instr::Eval(StackFn::Swap),
        ]});
    }

    #[test]
    fn jump() {
        let mut p = Process::default();

        let code = &[
            Instr::PushLit(Value::Int(25)),
            Instr::Write,
            Instr::PushLit(Value::Int(2)),
            Instr::PushLit(Value::Int(2)),
            Instr::Eval(StackFn::Add),
            Instr::PushVar(StackAddr(0)),
            Instr::Compare(Relation::Lesser),
            Instr::JumpIf(Label(0)),
            Instr::Hcf,
            Instr::Nop,
        ];

        let labels = &[
            Address(9),
        ];

        for _ in code {
            p.fetch(code).unwrap();
            p.exec(labels).unwrap();
        }
    }
}
