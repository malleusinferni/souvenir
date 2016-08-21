use std::collections::{HashMap, HashSet};

use ir::*;

pub struct Process {
    state: State,
    module: u32,
    id: u32,
    pc: u32,
    cond: u8,
    menu: Vec<(Label, String)>,
    buf: Vec<Val>,
    var: HashMap<u32, Val>,
    tmp: HashMap<u32, Val>,
    traps: HashSet<Label>,
}

pub struct Supervisor {
    atoms: HashMap<u32, String>,
    lists: HashMap<u32, Vec<Val>>,
    strings: HashMap<u32, String>,
    code: Vec<Op>,
    labels: HashMap<Label, u32>,
    modenv: HashMap<u32, ModEnv>,
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

pub type RunResult<T> = Result<T, RunErr>;

#[derive(Debug, Eq, PartialEq)]
pub enum RunErr {
    InstrOverflow(u32),
    Uninitialized(Reg),
    Unwritable(Reg),
    UnimplementedOp(Op),
    ArithOverflow,
    IllegalAdd(Val, Val),
}

impl Supervisor {
    pub fn run(&mut self, process: &mut Process) -> RunResult<()> {
        assert_eq!(process.state, State::Running);

        match try!(self.fetch(&mut process.pc)) {
            Op::Nop => (),

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

            Op::Add(lhs, rhs, dst) => {
                let lhs = try!(self.check_load(lhs, process));
                let rhs = try!(self.check_load(rhs, process));

                use ir::Val::*;

                let result = match (lhs, rhs) {
                    (Int(a), Int(b)) => match a.checked_add(b) {
                        Some(c) => Int(c),
                        None => return Err(RunErr::ArithOverflow),
                    },

                    _ => return Err(RunErr::IllegalAdd(lhs, rhs)),
                };

                try!(self.store(dst, process, result));
            },

            op => return Err(RunErr::UnimplementedOp(op)),
        }

        Ok(())
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
    fn new(id: u32) -> Self {
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
        }
    }
}
