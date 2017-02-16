use std::collections::{HashMap, VecDeque};

use string_interner::{StringInterner, NonNegative};

use vecmap::*;

/// Entry point to the interpreter API.
pub struct Scheduler {
    program: Program,

    /// Processes which are alive and ready to run immediately.
    queue: RunQueue,

    /// Buffer of processes presently being executed.
    workspace: VecDeque<(ActorId, Box<Process>)>,

    /// Buffer of input events presently being handled.
    inbuf: VecDeque<InSignal>,

    /// Buffered output from execution.
    outbuf: VecDeque<OutSignal>,

    next_pid: u32,

    next_event: u32,
}

/// Organizes processes by current status.
struct RunQueue {
    running: HashMap<ActorId, Box<Process>>,
    sleeping: HashMap<ActorId, (Tag, Box<Process>)>,
    dead: VecDeque<Box<Process>>,
}

/// Program data marshalled for use by the host environment.
#[derive(Clone, Debug)]
pub enum RawValue {
    ActorId(ActorId),
    Atom(String),
    Int(i32),
    Str(String),
    List(Vec<RawValue>),
}

/// Signals sent into the interpreter by the host environment. Cannot be cloned.
pub enum InSignal {
    Kill(ActorId),
    EndSay(SayReplyToken),
    EndAsk(AskReplyToken),
}

/// Signals sent from the interpreter to the host environment. Cannot be cloned.
pub enum OutSignal {
    Exit(ActorId),
    Hcf(ActorId, RunErr),
    Say(SayToken),
    Ask(AskToken),
}

/// Opaque key into the supervisor's list of processes.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActorId(u32);

#[derive(Debug, Eq, Hash, PartialEq)]
struct Tag(ActorId, u32);

// NB. No Copy, no Clone!
pub struct SayToken(Tag, RawValue);
pub struct SayReplyToken(Tag);
pub struct AskToken(Tag, Vec<(i32, RawValue)>);
pub struct AskReplyToken(Tag, i32);

/// Executable program
#[derive(Clone, Debug)]
pub struct Program {
    /// Instructions from all blocks.
    pub code: VecMap<InstrAddr, Instr>,

    /// Lookup table for the destinations of jump instructions.
    pub jump_table: VecMap<Label, InstrAddr>,

    /// Interned atoms.
    pub atom_table: StringInterner<AtomId>,

    /// Interned (global) string constants.
    pub str_table: StringInterner<StrId>,
}

/// Unencoded (immediately executable) VM instructions.
///
/// Multi-argument operations follow the convention of `input -> output` in
/// their arguments. So, for example, `Add(a, b)` reads a value from `a` and
/// adds it to `b`.
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
    True(Flag),
    False(Flag),
    LoadLit(Value, Reg),
    Alloc(ListLen, Reg),
    Read(Ptr, Reg),
    Write(Reg, Ptr),
    Jump(Label),
    JumpIf(Flag, Label),
    Recur(Reg, Label),
    Arm(Reg, Label),
    Disarm(Label),
    Return(bool),
    Blocking(Io),
    Nop,
    Bye,
    Hcf,
}

/// Instructions representing blocking IO operations.
#[derive(Copy, Clone, Debug)]
pub enum Io {
    GetPid(Reg),
    SendMsg(Reg, Reg),
    Roll(Reg, Reg),
    Sleep(f32),
    ArmAtomic(Reg, Label),
    Spawn(Reg, Label, Reg),
    Native(Reg, NativeFn, Reg),
    Say(Reg),
    Ask(Reg, Reg),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Int(i32),
    Atom(AtomId),
    ActorId(ActorId),
    StrConst(StrId),
    StrAddr(u32),
    ListAddr(HeapAddr),
    Capacity(u32),
    Undefined,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TypeTag {
    Int,
    Atom,
    Actor,
    Str,
    List,
}

#[derive(Copy, Clone, Debug)]
pub struct NativeFn(u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ListLen(pub u32);

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Ptr {
    addr: Reg,
    offset: u32,
}

pub type JumpTable = VecMap<Label, InstrAddr>;

pub struct StackFrame {
    gpr: [Value; REG_COUNT],
    flag: [bool; REG_COUNT],
}

/// Prototype for a message handler.
#[derive(Copy, Clone, Debug)]
pub struct Trap {
    label: Label,
    env: HeapAddr,
}

/// State of a handler invocation.
pub struct Continuation {
    /// Code position to return to once there are no handlers left to execute.
    return_addr: InstrAddr,

    /// Input to running handlers.
    message: Value,

    sender: Value,

    frame: StackFrame,

    /// Sequence of remaining handlers in this invocation.
    queue: Vec<Trap>,
}

pub struct Stack {
    lower: StackFrame,
    upper: Option<Continuation>,
}

#[derive(Clone, Debug, Default)]
pub struct Heap {
    values: Vec<Value>,
    strings: Vec<String>,
}

pub struct Process {
    stack: Stack,
    heap: Heap,
    traps: Vec<Trap>,
    op: Instr,
    pc: InstrAddr,
}

#[derive(Copy, Clone, Debug)]
pub enum RunErr {
    StackOverflow,
    StackUnderflow,
    NoSuchRegister(Reg),
    NoSuchFlag(Flag),
    NoSuchLabel(Label),
    FetchOutOfBounds(InstrAddr),
    IllegalInstr(Instr),
    UnallocatedAccess(usize),
    HeapCorrupted(Value),
    ListOutOfBounds(usize, u32),
    TypeMismatch(Value, TypeTag),
    DividedByZero,
    Unrepresentable(usize),
    Uninitialized,
    NoSuchAtom(AtomId),
    NoSuchValue(Value),
}

pub type Ret<T> = Result<T, RunErr>;

const REG_COUNT: usize = 0x400;

impl Default for Instr {
    fn default() -> Self { Instr::Nop }
}

impl Default for StackFrame {
    fn default() -> Self {
        StackFrame {
            gpr: [Value::Int(-1); REG_COUNT],
            flag: [false; REG_COUNT],
        }
    }
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

index_via_u32!(Label, InstrAddr);
index_via_u32!(InstrAddr, Instr);
index_via_u32!(Reg, Value);
index_via_u32!(HeapAddr, Value);
index_via_u32!(Flag, bool);

macro_rules! symbol_via_u32 {
    ( $name:ident ) => {
        #[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub u32);

        impl From<$name> for usize {
            fn from($name(u): $name) -> Self {
                u as usize
            }
        }

        impl From<usize> for $name {
            fn from(u: usize) -> Self {
                $name(u as u32)
            }
        }

        impl NonNegative for $name { }

        // Blanket implementation automatically satisfied
        //impl Symbol for $name { }
    };
}

symbol_via_u32!(AtomId);
symbol_via_u32!(StrId);

impl Stack {
    fn current(&mut self) -> &mut StackFrame {
        if let Some(c) = self.upper.as_mut() {
            return &mut c.frame;
        }

        &mut self.lower
    }

    fn push(&mut self, cc: Continuation) -> Ret<()> {
        if self.upper.is_some() {
            Err(RunErr::StackOverflow)
        } else {
            self.upper = Some(cc);
            Ok(())
        }
    }

    fn pop(&mut self) -> Ret<Continuation> {
        self.upper.take().ok_or(RunErr::StackUnderflow)
    }
}

impl StackFrame {
    fn get(&self, r: Reg) -> Ret<Value> {
        let i = r.0 as usize;
        if i < REG_COUNT {
            Ok(self.gpr[i])
        } else {
            Err(RunErr::NoSuchRegister(r))
        }
    }

    fn set(&mut self, r: Reg, v: Value) -> Ret<()> {
        let i = r.0 as usize;
        if i < REG_COUNT {
            self.gpr[i] = v;
            Ok(())
        } else {
            Err(RunErr::NoSuchRegister(r))
        }
    }

    fn get_flag(&mut self, f: Flag) -> Ret<bool> {
        let i = f.0 as usize;
        if i < REG_COUNT {
            Ok(self.flag[i])
        } else {
            Err(RunErr::NoSuchFlag(f))
        }
    }

    fn set_flag(&mut self, f: Flag, v: bool) -> Ret<()> {
        let i = f.0 as usize;
        if i < REG_COUNT {
            self.flag[i] = v;
            Ok(())
        } else {
            Err(RunErr::NoSuchFlag(f))
        }
    }
}

impl Heap {
    fn alloc(&mut self, len: ListLen) -> Ret<HeapAddr> {
        let addr = HeapAddr(self.values.len() as u32);
        self.values.push(len.into());
        for _ in 0 .. len.0 {
            self.values.push(Value::Undefined);
        }
        Ok(addr)
    }

    fn size_of(&self, addr: HeapAddr) -> Ret<u32> {
        let addr: usize = addr.into();
        let header = *self.values.get(addr)
            .ok_or(RunErr::UnallocatedAccess(addr))?;

        if let Value::Capacity(size) = header {
            Ok(size)
        } else {
            Err(RunErr::HeapCorrupted(header))
        }
    }

    fn check_bounds(&self, addr: HeapAddr, offset: u32) -> Ret<usize> {
        if self.size_of(addr)? > offset {
            Ok(usize::from(addr) + 1 + offset as usize)
        } else {
            Err(RunErr::ListOutOfBounds(usize::from(addr), offset))
        }
    }

    fn get(&self, addr: HeapAddr, offset: u32) -> Ret<Value> {
        let i = self.check_bounds(addr, offset)?;
        Ok(self.values[i])
    }

    fn set(&mut self, addr: HeapAddr, offset: u32, value: Value) -> Ret<()> {
        let i = self.check_bounds(addr, offset)?;
        self.values[i] = value;
        Ok(())
    }
}

impl Process {
    pub fn exec(&mut self, program: &Program) -> Ret<()> {
        match self.op {
            Instr::Nop => (),

            Instr::Cpy(src, dst) => {
                let frame = self.stack.current();
                let value = frame.get(src)?;
                frame.set(dst, value)?;
            },

            Instr::Add(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs + rhs).into())?;
            },

            Instr::Sub(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs - rhs).into())?;
            },

            Instr::Div(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                if rhs == 0 {
                    return Err(RunErr::DividedByZero);
                } else {
                    frame.set(dst, (lhs / rhs).into())?;
                }
            },

            Instr::Mul(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get(dst)?.as_int()?;
                let rhs = frame.get(src)?.as_int()?;
                frame.set(dst, (lhs * rhs).into())?;
            },

            Instr::Eql(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?;
                let rhs = frame.get(rhs)?;
                frame.set_flag(flag, lhs == rhs)?;
            },

            Instr::Gte(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs >= rhs)?;
            },

            Instr::Lte(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs <= rhs)?;
            },

            Instr::Gt(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs > rhs)?;
            },

            Instr::Lt(lhs, rhs, flag) => {
                let frame = self.stack.current();
                let lhs = frame.get(lhs)?.as_int()?;
                let rhs = frame.get(rhs)?.as_int()?;
                frame.set_flag(flag, lhs < rhs)?;
            },

            Instr::And(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get_flag(dst)?;
                let rhs = frame.get_flag(src)?;
                frame.set_flag(dst, lhs && rhs)?;
            },

            Instr::Or(src, dst) => {
                let frame = self.stack.current();
                let lhs = frame.get_flag(dst)?;
                let rhs = frame.get_flag(src)?;
                frame.set_flag(dst, lhs || rhs)?;
            },

            Instr::Not(flag) => {
                let frame = self.stack.current();
                let value = frame.get_flag(flag)?;
                frame.set_flag(flag, !value)?;
            },

            Instr::True(flag) => {
                self.stack.current().set_flag(flag, true)?;
            },

            Instr::False(flag) => {
                self.stack.current().set_flag(flag, false)?;
            },

            Instr::LoadLit(value, dst) => {
                self.stack.current().set(dst, value)?;
            },

            Instr::Alloc(len, dst) => {
                let addr = self.heap.alloc(len)?;
                self.stack.current().set(dst, addr.into())?;
            },

            Instr::Read(ptr, dst) => {
                let frame = self.stack.current();
                let addr = frame.get(ptr.addr)?.as_addr()?;
                let value = self.heap.get(addr, ptr.offset)?;
                frame.set(dst, value)?;
            },

            Instr::Write(src, ptr) => {
                let frame = self.stack.current();
                let value = frame.get(src)?;
                let addr = frame.get(ptr.addr)?.as_addr()?;
                self.heap.set(addr, ptr.offset, value)?;
            },

            Instr::Jump(label) => {
                self.pc = *program.jump_table.get(label)?;
            },

            Instr::JumpIf(flag, label) => {
                if self.stack.current().get_flag(flag)? {
                    self.pc = *program.jump_table.get(label)?;
                }
            },

            Instr::Return(finished) => {
                let cc = self.stack.pop()?;
                self.pc = cc.return_addr;

                if !finished {
                    self.call(cc, program)?;
                }
            },

            Instr::Arm(env, label) => {
                self.arm(env, label)?;
            },

            Instr::Disarm(label) => {
                self.traps.retain(|trap| trap.label != label);
            },

            Instr::Recur(argv, label) => {
                unimplemented!()
            },

            Instr::Blocking(_) | Instr::Bye | Instr::Hcf => {
                return Err(RunErr::IllegalInstr(self.op))
            },
        }

        Ok(())
    }

    fn arm(&mut self, env: Reg, label: Label) -> Ret<()> {
        self.traps.retain(|trap| trap.label != label);
        let addr = self.stack.current().get(env)?.as_addr()?;
        self.traps.push(Trap {
            env: addr,
            label: label,
        });
        Ok(())
    }

    fn call(&mut self, mut cc: Continuation, program: &Program) -> Ret<()> {
        let trap = match cc.queue.pop() {
            Some(trap) => trap,
            None => return Ok(()),
        };

        cc.frame.gpr[0] = trap.env.into();
        cc.frame.gpr[1] = cc.message;
        cc.frame.gpr[2] = cc.sender;

        self.stack.push(cc)?;
        self.pc = *program.jump_table.get(trap.label)?;

        Ok(())
    }

    pub fn fetch(&mut self, program: &Program) -> Ret<()> {
        self.op = *program.code.get(self.pc)?;
        self.pc.0 += 1;
        Ok(())
    }

    fn is_blocked(&self) -> Ret<bool> {
        match self.op {
            Instr::Hcf => Err(RunErr::IllegalInstr(self.op)),

            Instr::Bye | Instr::Blocking(_) => {
                Ok(true)
            },

            _ => Ok(false),
        }
    }

    fn run(&mut self, program: &Program) -> Ret<bool> {
        const SOME_SMALL_NUMBER: usize = 100;

        for _ in 0 .. SOME_SMALL_NUMBER {
            self.exec(program)?;

            if self.is_blocked()? {
                return Ok(true);
            }

            self.fetch(program)?;
        }

        Ok(false)
    }

    fn write_reg(&mut self, r: Reg, v: Value) -> Ret<()> {
        self.stack.current().set(r, v)
    }
}

impl Scheduler {
    pub fn send<I: IntoIterator<Item=InSignal>>(&mut self, inbuf: I) {
        self.inbuf.extend(inbuf.into_iter());

        for event in self.inbuf.drain(..) {
            unimplemented!()
        }
    }

    pub fn dispatch(&mut self) {
        // FIXME: This isn't a very good scheduler.

        self.workspace.extend(self.queue.running.drain());

        while let Some((id, mut process)) = self.workspace.pop_front() {
            match self.run(id, &mut process) {
                Ok(Some(tag)) => {
                    self.queue.sleeping.insert(id, (tag, process));
                },

                Ok(None) => {
                    self.queue.running.insert(id, process);
                },

                Err(err) => {
                    self.outbuf.push_back(OutSignal::Hcf(id, err));
                    self.queue.dead.push_back(process);
                },
            }
        }
    }

    fn run(&mut self, id: ActorId, process: &mut Process) -> Ret<Option<Tag>> {
        if process.run(&self.program)? {
            return Ok(None);
        }

        match process.op.io()? {
            Io::GetPid(dst) => {
                let pid = Value::ActorId(id);
                process.stack.current().set(dst, pid)?;
                process.fetch(&self.program)?;
                Ok(None)
            },

            Io::Say(msg) => {
                let value = process.stack.current().get(msg)?;
                let content = self.marshal(&process.heap, value)?;
                let tag = self.tag(id);
                let token = SayToken(tag.private_clone(), content);
                self.outbuf.push_back(token.into());
                Ok(Some(tag))
            },

            Io::Ask(src, dst) => {
                let value = process.stack.current().get(src)?;
                let choices = self.get_menu(&process.heap, value)?;
                let tag = self.tag(id);
                let token = AskToken(tag.private_clone(), choices);
                self.outbuf.push_back(token.into());
                Ok(Some(tag))
            },

            Io::ArmAtomic(env, label) => {
                process.arm(env, label)?;
                let tag = self.tag(id);
                Ok(Some(tag))
            },

            Io::Native(_, _, _) => {
                unimplemented!()
            },

            Io::Roll(_, _) => {
                unimplemented!()
            },

            Io::SendMsg(_, _) => {
                unimplemented!()
            },

            Io::Sleep(time) => {
                unimplemented!()
            },

            Io::Spawn(argv, label, dst) => {
                let argv = process.stack.current().get(argv)?;
                let mut new = self.queue.fetch();
                let new_id = ActorId(self.next_pid);
                self.next_pid += 1;
                process.stack.current().set(dst, Value::ActorId(new_id))?;
                // FIXME: Copy arguments
                // FIXME: Jump to entry point
                process.fetch(&self.program)?;
                Ok(None)
            },
        }
    }

    fn tag(&mut self, id: ActorId) -> Tag {
        let tag = Tag(id, self.next_event);
        self.next_event += 1;
        tag
    }

    fn marshal(&self, heap: &Heap, value: Value) -> Ret<RawValue> {
        match value {
            Value::Int(i) => Ok(RawValue::Int(i)),
            Value::ActorId(id) => Ok(RawValue::ActorId(id)),

            Value::Atom(id) => {
                match self.program.atom_table.resolve(id) {
                    Some(s) => Ok(RawValue::Atom(s.to_owned())),
                    None => Err(RunErr::NoSuchAtom(id)),
                }
            },

            Value::StrAddr(addr) => {
                match heap.strings.get(addr as usize) {
                    Some(s) => Ok(RawValue::Str(s.clone())),
                    None => Err(RunErr::UnallocatedAccess(addr as usize)),
                }
            },

            Value::StrConst(id) => {
                match self.program.str_table.resolve(id) {
                    Some(s) => Ok(RawValue::Str(s.to_owned())),
                    None => Err(RunErr::NoSuchValue(value)),
                }
            },

            Value::ListAddr(addr) => {
                let len = heap.size_of(addr)?;
                let mut list = Vec::with_capacity(len as usize);
                for i in 0 .. len {
                    let value = heap.get(addr, i)?;
                    list.push(self.marshal(heap, value)?);
                }
                Ok(RawValue::List(list))
            },

            Value::Capacity(_) => Err(RunErr::HeapCorrupted(value)),
            Value::Undefined => Err(RunErr::Uninitialized),
        }
    }

    fn get_menu(&self, heap: &Heap, value: Value) -> Ret<Vec<(i32, RawValue)>> {
        let addr = value.as_addr()?;
        let len = heap.size_of(addr)?;
        let mut menu = Vec::with_capacity(len as usize);
        for i in 0 .. len {
            let choice_addr = heap.get(addr, i)?.as_addr()?;
            let test = heap.get(choice_addr, 0)?.as_bool()?;
            let tag = heap.get(choice_addr, 1)?.as_int()?;
            let title = heap.get(choice_addr, 2)?;
            let title = self.marshal(heap, title)?;
            if test { menu.push((tag, title)); }
        }
        Ok(menu)
    }
}

impl Instr {
    fn io(self) -> Ret<Io> {
        match self {
            Instr::Blocking(io) => Ok(io),
            _ => Err(RunErr::IllegalInstr(self)),
        }
    }
}

impl RunQueue {
    fn find_mut(&mut self, id: ActorId) -> Option<&mut Process> {
        if let Some(process) = self.running.get_mut(&id) {
            return Some(process.as_mut());
        }

        if let Some(pair) = self.sleeping.get_mut(&id) {
            return Some(pair.1.as_mut());
        }

        None
    }

    fn fetch(&mut self) -> Box<Process> {
        if let Some(old) = self.dead.pop_front() {
            old
        } else {
            Box::new(Process::default())
        }
    }
}

impl Value {
    pub fn tag(&self) -> Ret<TypeTag> {
        Ok(match self {
            &Value::Int(_) => TypeTag::Int,
            &Value::Atom(_) => TypeTag::Atom,
            &Value::ActorId(_) => TypeTag::Actor,
            &Value::StrConst(_) | &Value::StrAddr(_) => TypeTag::Str,
            &Value::ListAddr(_) | &Value::Capacity(_) => TypeTag::List,
            &Value::Undefined => return Err(RunErr::Uninitialized),
        })
    }

    pub fn as_int(self) -> Ret<i32> {
        match self {
            Value::Int(i) => Ok(i),
            _ => Err(RunErr::TypeMismatch(self, TypeTag::Int)),
        }
    }

    pub fn as_bool(self) -> Ret<bool> {
        Ok(self.as_int()? != 0)
    }

    fn as_addr(self) -> Ret<HeapAddr> {
        match self {
            Value::ListAddr(addr) => Ok(addr),
            _ => Err(RunErr::TypeMismatch(self, TypeTag::List)),
        }
    }
}

impl Tag {
    fn private_clone(&self) -> Self {
        Tag(self.0, self.1)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i)
    }
}

impl From<ListLen> for Value {
    fn from(len: ListLen) -> Self {
        Value::Capacity(len.0)
    }
}

impl From<HeapAddr> for Value {
    fn from(addr: HeapAddr) -> Self {
        Value::ListAddr(addr)
    }
}

impl From<IndexErr<Label>> for RunErr {
    fn from(err: IndexErr<Label>) -> Self {
        match err {
            IndexErr::OutOfBounds(k) => RunErr::NoSuchLabel(k),
            IndexErr::ReprOverflow(u) => RunErr::Unrepresentable(u),
        }
    }
}

impl From<IndexErr<InstrAddr>> for RunErr {
    fn from(err: IndexErr<InstrAddr>) -> Self {
        match err {
            IndexErr::OutOfBounds(k) => RunErr::FetchOutOfBounds(k),
            IndexErr::ReprOverflow(u) => RunErr::Unrepresentable(u),
        }
    }
}

impl From<SayToken> for OutSignal {
    fn from(token: SayToken) -> Self {
        OutSignal::Say(token)
    }
}

impl From<AskToken> for OutSignal {
    fn from(token: AskToken) -> Self {
        OutSignal::Ask(token)
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack {
            lower: StackFrame::default(),
            upper: None,
        }
    }
}

impl Default for Process {
    fn default() -> Self {
        Process {
            stack: Stack::default(),
            heap: Heap::default(),
            traps: vec![],
            op: Instr::Nop,
            pc: InstrAddr(0),
        }
    }
}
