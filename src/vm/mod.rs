pub mod eval;

use vecmap::*;

#[derive(Clone, Debug)]
pub struct Program {
    /// String constants in shared memory.
    pub strings: VecMap<StrId, String>,

    /// Instructions from all blocks.
    pub code: VecMap<InstrAddr, Instr>,

    /// Lookup table for the destinations of jump instructions.
    pub jump_table: VecMap<Label, InstrAddr>,

    /// Lookup table for function calling conventions.
    pub funcdefs: VecMap<Label, FuncDef>,

    /// Pre-evaluated environments for module-scoped variables.
    pub modenvs: VecMap<ModuleID, eval::Process>,
}

#[derive(Copy, Clone, Debug)]
pub enum Instr {
    Cpy(Reg, Reg),
    Add(Reg, Reg),
    Sub(Reg, Reg),
    Div(Reg, Reg),
    Mul(Reg, Reg),
    Eql(Reg, Reg, Flag),
    Gte(Reg, Reg, Flag),
    Lte(Reg, Reg, Flag),
    Gt(Reg, Reg, Flag),
    Lt(Reg, Reg, Flag),
    And(Flag, Flag),
    Or(Flag, Flag),
    Not(Flag),
    LoadLit(Value, Reg),
    Alloc(ListLen, Reg),
    Read(ListRef, Reg),
    Write(Reg, ListRef),
    Jump(Label),
    JumpIf(Label, Flag),
    Spawn(Reg, Label, Reg),
    Recur(Reg, Label),
    Native(Reg, NativeFn, Reg),
    SendMsg(Reg, Reg),
    Sleep(f32),
    Arm(Reg, Label),
    Disarm(Label),
    Return(bool),
    Nop,
    Bye,
    Hcf,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Atom(u32),
    ActorId(u32),
    StrConst(StrId),
    StrAddr(u32),
    ListAddr(u32),
    ListHeader(u32),
}

#[derive(Copy, Clone, Debug)]
pub enum NativeFn {
    Roll,
    GetPid,
    Custom(u32),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ListLen(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ListRef {
    ptr: Reg,
    offset: u32,
}

pub type JumpTable = VecMap<Label, InstrAddr>;

#[derive(Copy, Clone, Debug)]
pub struct StackAddr(u32);

#[derive(Copy, Clone, Debug)]
pub struct FuncDef {
    module: ModuleID,
    argcount: u32,
}

impl Default for Instr {
    fn default() -> Self { Instr::Nop }
}

macro_rules! index_via_u32 {
    ( $name:ident, $( $value:ty ),* ) => {
        #[derive(Copy, Clone, Debug, Default, PartialEq)]
        pub struct $name(pub u32);

        impl From<$name> for usize {
            fn from($name(u): $name) -> Self {
                u as usize
            }
        }

        impl CheckedFrom<usize> for $name {
            fn checked_from(u: usize) -> Option<Self> {
                if u > u32::max_value() as usize {
                    None
                } else {
                    Some($name(u as u32))
                }
            }
        }

        $( impl IndexFor<$value> for $name {} )*
    };
}

index_via_u32!(Label, InstrAddr, ModuleID, FuncDef);
index_via_u32!(InstrAddr, Instr);
index_via_u32!(Reg, Value);
index_via_u32!(Flag, bool);
index_via_u32!(StrId, String);
index_via_u32!(ModuleID, eval::Process);

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
            Instr::Jump(Label(0)),
        ];

        let labels = &[
            InstrAddr(9),
        ];

        for _ in code {
            p.exec(labels).unwrap();
            p.fetch(code).unwrap();
        }
    }
}
