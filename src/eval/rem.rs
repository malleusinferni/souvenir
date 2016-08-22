use std::collections::{HashMap, HashSet};

use ir::*;

pub struct Process {
    state: State,
    module: u32,
    id: ActorID,
    pc: u32,
    cond: u8,
    menu: Vec<(Label, String)>,
    buf: Vec<Val>,
    var: HashMap<u32, Val>,
    tmp: HashMap<u32, Val>,
    traps: HashSet<Label>,
    visits: HashMap<Label, i32>,
}

pub struct Supervisor {
    atoms: HashMap<u32, String>,
    strings: HashMap<u32, String>,
    code: Vec<Op>,
    labels: HashMap<Label, usize>,
    modenv: HashMap<u32, ModEnv>,
    outbox: Vec<Message>,
}

mod bits {
    pub const ZERO: u8 = 0b0000_0001;
    pub const POS: u8 = 0b0000_0010;
    pub const NEG: u8 = 0b0000_0100;
}

#[derive(Debug, Eq, PartialEq)]
enum State {
    Running,
    BlockedOnOutput,
    BlockedOnInput,
    Sleeping(u32),
    OnFire(RunErr),
    Dead,
}

pub struct Message {
    src: ActorID,
    dst: ActorID,
    body: Vec<Val>,
}

pub type RunResult<T> = Result<T, RunErr>;

#[derive(Debug, Eq, PartialEq)]
pub enum RunErr {
    InstrOverflow(u32),
    Uninitialized(Reg),
    Unwritable(Reg),
    UnimplementedOp(Op),
    UnimplementedBinop(Binop),
    ArithOverflow,
    IllegalAdd,
    IllegalCmp,
    NotAnActor,
    UserErr,
}

impl Supervisor {
    pub fn run(&mut self, process: &mut Process) -> RunResult<()> {
        assert_eq!(process.state, State::Running);

        match try!(self.fetch(&mut process.pc)) {
            Op::Nop => (),

            Op::Binop(bop, lhs, rhs, dst) => {
                let lhs = try!(self.check_load(lhs, process));
                let rhs = try!(self.check_load(rhs, process));

                use ir::Binop::*;
                use ir::Val::*;

                let result = match (bop, lhs, rhs) {
                    (Add, Int(a), Int(b)) => match a.checked_add(b) {
                        Some(c) => Int(c),
                        None => return Err(RunErr::ArithOverflow),
                    },

                    (Add, List(mut xs), x) => {
                        xs.push(x);
                        List(xs)
                    },

                    (Add, Strseq(mut xs), x) => {
                        xs.push(x);
                        Strseq(xs)
                    },

                    (Add, _, _) => return Err(RunErr::IllegalAdd),

                    _ => return Err(RunErr::UnimplementedBinop(bop)),
                };

                try!(self.store(dst, process, result));
            },

            Op::LitInt(n, dst) => {
                try!(self.store(dst, process, Val::Int(n)));
            },

            Op::LitStr(s, dst) => {
                try!(self.store(dst, process, Val::Strid(s)));
            },

            Op::Mov(src, dst) => {
                let val = try!(self.check_load(src, process));
                try!(self.store(dst, process, val));
            },

            Op::Phi(lhs, rhs, dst) => {
                let lhs = try!(self.load(lhs, process));
                let rhs = try!(self.load(rhs, process));
                match (lhs, rhs) {
                    (Some(val), None) => try!(self.store(dst, process, val)),
                    (None, Some(val)) => try!(self.store(dst, process, val)),
                    (None, None) => return Err(RunErr::Uninitialized(dst)),
                    _ => return Err(RunErr::Uninitialized(dst)),
                }
            },

            Op::Cmp(lhs, rhs) => {
                let lhs = try!(self.check_load(lhs, process));
                let rhs = try!(self.check_load(rhs, process));

                use ir::Val::*;

                match (lhs, rhs) {
                    (Int(m), Int(n)) => process.compare(m - n),
                    
                    _ => return Err(RunErr::IllegalCmp),
                }
            },

            Op::Test(src) => {
                let val = try!(self.check_load(src, process));

                use ir::Val::*;

                match val {
                    Int(n) => process.compare(n),

                    _ => return Err(RunErr::IllegalCmp),
                }
            },

            Op::Bool(cond, dst) => {
                let val = Val::Int(if process.test(cond) { 1 } else { 0 });
                try!(self.store(dst, process, val));
            },

            Op::Not(cond) => {
                match cond {
                    Cond::True => (),
                    Cond::Zero => process.cond ^= bits::ZERO,
                    Cond::Negative => process.cond ^= bits::NEG,
                    Cond::Positive => process.cond ^= bits::POS,
                }
            },

            Op::Untemp => {
                process.tmp.clear();
            },

            Op::Msg(dst) => {
                let dst = try!(self.check_load(dst, process));
                let recip_id = match dst {
                    Val::Aid(id) => id,
                    _ => return Err(RunErr::NotAnActor),
                };

                let body = process.buf.drain(..).collect();

                self.outbox.push(Message {
                    src: process.id,
                    dst: recip_id,
                    body: body,
                });
            },

            Op::Write => { process.state = State::BlockedOnOutput; },

            Op::Read => { process.state = State::BlockedOnInput; },

            Op::AddMenu(label) => {
                let mut title = String::new();
                for val in process.buf.drain(..) {
                    title.push_str(&val.to_string());
                }
                process.menu.push((label, title));
            },

            Op::CheckMenu => {
                let n = process.menu.len() as i32;
                process.compare(n);
            },

            Op::Push(src) => {
                let val = try!(self.check_load(src, process));
                process.buf.push(val);
            },

            Op::Nil => { process.buf.clear(); }

            Op::Jump(cond, Label(l)) => {
                if process.test(cond) {
                    process.pc = l;
                }
            },

            Op::Count(label, dst) => {
                let count = process.visits.get(&label).cloned().unwrap_or(0);
                try!(self.store(dst, process, Val::Int(count)));
            },

            Op::Spawn(label, dst) => {
                let args = process.buf.drain(..).collect();
                let pid = try!(self.spawn(label, args));
                try!(self.store(dst, process, Val::Aid(pid)));
            },

            Op::Tail(label) => {
                let args: Vec<_> = process.buf.drain(..).collect();

                // TODO: Search for label
                // TODO: Make sure label accepts args
                // TODO: Check arg length

                process.tmp.clear();
                process.var.clear();
                process.traps.clear();

                // TODO: Bind args to var names

                process.pc = label.0;

                unimplemented!();
            },

            // TODO: Op::Trap

            // TODO: Op::Untrap

            Op::Bye => { process.state = State::Dead; }

            Op::Hcf => { process.state = State::OnFire(RunErr::UserErr); }

            op => return Err(RunErr::UnimplementedOp(op)),
        }

        Ok(())
    }

    pub fn spawn(&self, label: Label, args: Vec<Val>) -> RunResult<ActorID> {
        unimplemented!()
    }

    fn fetch(&self, pc: &mut u32) -> RunResult<Op> {
        let old = *pc as usize;
        pc.checked_add(1).and_then(|new| {
            *pc = new;
            self.code.get(old)
        }).cloned().ok_or(RunErr::InstrOverflow(*pc))
    }

    fn check_load(&self, reg: Reg, process: &Process) -> RunResult<Val> {
        self.load(reg, process).and_then(|o| {
            o.ok_or(RunErr::Uninitialized(reg))
        })
    }

    fn load(&self, reg: Reg, process: &Process) -> RunResult<Option<Val>> {
        match reg {
            Reg::Var(v) => Ok(process.var.get(&v).cloned()),

            Reg::Tmp(t) => Ok(process.tmp.get(&t).cloned()),

            Reg::Mod(m) => { unimplemented!() },

            Reg::MyAid => Ok(Some(Val::Aid(process.id))),

            Reg::Discard => Err(RunErr::Uninitialized(reg)),
        }
    }

    fn store(&self, reg: Reg, process: &mut Process, val: Val) -> RunResult<()> {
        match reg {
            Reg::Var(v) => if process.var.contains_key(&v) {
                Err(RunErr::Unwritable(reg))
            } else {
                process.var.insert(v, val); Ok(())
            },

            Reg::Tmp(t) => if process.tmp.contains_key(&t) {
                Err(RunErr::Unwritable(reg))
            } else {
                process.tmp.insert(t, val); Ok(())
            },

            Reg::MyAid | Reg::Mod(_) => Err(RunErr::Unwritable(reg)),

            Reg::Discard => Ok(()),
        }
    }
}

pub enum ModEnv {
    Unevaluated(Label),
    Evaluated(HashMap<u32, Val>),
}

impl Process {
    fn new(id: ActorID) -> Self {
        Process {
            id: id,

            module: 0,
            state: State::BlockedOnInput,
            pc: 0,
            cond: 0,
            menu: vec![],
            buf: vec![],
            var: HashMap::with_capacity(32),
            tmp: HashMap::with_capacity(32),
            traps: HashSet::with_capacity(8),
            visits: HashMap::with_capacity(32),
        }
    }

    fn compare(&mut self, n: i32) {
        self.cond = 0;
        if n == 0 { self.cond |= bits::ZERO; }
        if n < 0 { self.cond |= bits::NEG; }
        if n > 0 { self.cond |= bits::POS; }
    }

    fn test(&self, cond: Cond) -> bool {
        0 != match cond {
            Cond::True => 1,
            Cond::Zero => self.cond & bits::ZERO,
            Cond::Negative => self.cond & bits::NEG,
            Cond::Positive => self.cond & bits::POS,
        }
    }
}

impl Val {
    fn to_string(&self) -> String {
        unimplemented!()
    }
}
