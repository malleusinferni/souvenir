//! Defines an AST evaluator.

use std::collections::{HashMap, VecDeque};

use ast::*;

pub struct Process {
    id: ActorID,
    state: RunState,
    env: Env,
    traps: VecDeque<(Env, Trap)>,
    instructions: VecDeque<Stmt>,
}

pub type Env = HashMap<String, Expr>;

#[derive(Clone, Debug)]
pub enum RunState {
    Running,
    Sleeping(f32),
    Idling,
    OnFire(RuntimeError),
    WaitingForInput(Vec<(String, Vec<Stmt>)>),
    ProducedOutput(String),
    SelfTerminated,
}

#[derive(Clone, Debug)]
pub enum RuntimeError {
    Unimplemented(Stmt),
    IrreducibleExpr(Expr),
    NoSuchBinding(String),
    NoSuchModule,
    NoSuchKnot(Label),
    WrongNumberOfArgs(usize, usize),
    IllegalBinop(Expr, Binop, Expr),
    IllegalLvalue(Expr),
    DivideByZero(Expr),
}

pub type EvalResult<T> = Result<T, RuntimeError>;

pub struct Message {
    src: ActorID,
    dst: ActorID,
    body: Expr,
}

pub struct Evaluator {
    modules: HashMap<String, Module>,
    processes: Vec<Process>,
    messages: VecDeque<Message>,
    clockspeed: f32,
    next_actor_id: ActorID,
}

pub type ExecResult<T> = Result<T, String>;

impl Evaluator {
    pub fn new(speed: f32) -> Self {
        Evaluator {
            modules: HashMap::new(),
            processes: Vec::new(),
            messages: VecDeque::new(),
            clockspeed: speed,
            next_actor_id: ActorID(0),
        }
    }

    pub fn compile(&mut self, name: &str, source: &str) -> ExecResult<()> {
        use parser::parse_Module;
        use tokenizer::Tokenizer;

        if self.modules.len() > 0 {
            return Err(format!("Multiple modules not yet supported"));
        }

        let input = Tokenizer::new(source, 0);
        let module = match parse_Module(source, input) {
            Ok(code) => code,
            Err(e) => return Err(format!("Parse error: {:?}", e)),
        };

        self.modules.insert(name.to_owned(), module);

        Ok(())
    }

    pub fn spawn(&mut self, label: Label, args: Vec<Expr>) -> ExecResult<()> {
        let mut process = Process::new(self.next_actor_id.bump());

        let knot = try!(self.find_knot(label));
        try!(process.exec(knot, args));
        self.processes.push(process);

        Ok(())
    }

    pub fn dispatch(&mut self, timeslice: f32) -> RunState {
        let steps = (timeslice * self.clockspeed) as usize;
        let sleep_step = 1.0 / self.clockspeed;

        let mut process_queue = Vec::new();
        let mut message_queue = VecDeque::new();
        let mut main_process_state = None;

        for _ in 0 .. steps {
            process_queue.append(&mut self.processes);

            for mut process in process_queue.drain(..) {
                // FIXME: This is inelegant, to say the least
                message_queue.append(&mut self.messages);
                while let Some(msg) = message_queue.pop_front() {
                    if msg.dst != process.id {
                        self.messages.push_back(msg);
                        continue;
                    }

                    self.deliver(msg, &mut process);
                }

                match process.state.clone() {
                    RunState::SelfTerminated => {
                        // Process is dropped and forgotten!
                        continue;
                    },

                    RunState::OnFire(_) => {
                        // Keep process in queue, but don't execute
                        self.processes.push(process);
                        continue;
                    },

                    RunState::Sleeping(n) => {
                        process.state = RunState::Sleeping(n - sleep_step);
                    }

                    _ => (),
                }

                self.run_once(&mut process)
                    .unwrap_or_else(|e| process.hcf(e));

                if process.id.0 == 0 {
                    main_process_state = Some(process.state.clone());
                }

                self.processes.push(process);
            }
        }

        main_process_state.expect("Main process disappeared")
    }

    fn run_once(&mut self, process: &mut Process) -> EvalResult<()> {
        let stmt = match process.instructions.pop_front() {
            Some(stmt) => stmt,
            None => { process.state = RunState::Idling; return Ok(()) },
        };

        match stmt {
            Stmt::Empty => (),

            Stmt::Trace(expr) => {
                let output = try!(process.eval(expr)).to_string();
                process.state = RunState::ProducedOutput(output);
            },

            Stmt::SendMsg(dst, expr) => {
                let body = try!(process.eval(expr));

                let dst = match try!(process.eval(dst)) {
                    Expr::Actor(aid) => aid,
                    val => return Err(RuntimeError::IllegalLvalue(val)),
                };

                self.messages.push_back(Message {
                    src: process.id.clone(),
                    dst: dst,
                    body: body,
                });
            },

            Stmt::Trap(_, traps) => for trap in traps {
                let env = process.env.clone();
                process.traps.push_front((env, trap));
            },

            Stmt::TailCall(label, args) => {
                // Do this first so they evaluate in the current env
                let mut arg_values = Vec::with_capacity(args.len());
                for arg in args {
                    arg_values.push(try!(process.eval(arg)));
                }

                let knot = try!(self.find_knot(label));
                try!(process.exec(knot, arg_values));
                // It SHOULD be that easy... right?
            },

            other_stmt => {
                process.state = RunState::OnFire({
                    RuntimeError::Unimplemented(other_stmt)
                });
            },
        }

        Ok(())
    }

    fn find_knot(&self, label: Label) -> EvalResult<Knot> {
        let module: &Module = match self.modules.values().next() {
            Some(m) => m,
            None => return Err(RuntimeError::NoSuchModule),
        };

        for knot in module.knots.iter() {
            if knot.name == label {
                return Ok(knot.clone());
            }
        }

        Err(RuntimeError::NoSuchKnot(label))
    }

    fn deliver(&mut self, message: Message, process: &mut Process) {
        use std::mem;

        let traps = process.traps.clone();
        for (mut env, trap) in traps {
            mem::swap(&mut env, &mut process.env);

            match process.bind(trap.pattern, message.body.clone()) {
                Ok(true) => (),
                _ => { process.env = env; return; },
            };

            let src = Expr::Actor(message.src.clone());
            match process.bind(trap.origin, src) {
                Ok(true) => (),
                _ => { process.env = env; return; },
            };

            match process.eval(trap.guard) {
                Ok(expr) => match expr.truthiness() {
                    Ok(true) => (),
                    _ => { process.env = env; return; },
                },
                _ => { process.env = env; return; },
            };

            process.traps.clear();
            process.instructions.clear();
            process.instructions.extend(trap.body.into_iter());
            return;
        }
    }
}

impl Process {
    fn new(id: ActorID) -> Self {
        let mut env = HashMap::new();
        env.insert("Self".to_owned(), Expr::Actor(id.clone()));

        Process {
            id: id,
            env: env,
            state: RunState::Running,
            traps: VecDeque::new(),
            instructions: VecDeque::new(),
        }
    }

    fn bind(&mut self, name: Expr, value: Expr) -> EvalResult<bool> {
        if !value.is_self_evaluating() {
            return Err(RuntimeError::IrreducibleExpr(value));
        }

        match name {
            Expr::Hole => Ok(true), // Binds nothing but matches anything

            Expr::Var(ref name) if !self.env.contains_key(name) => {
                // Name is not present, so insert it
                self.env.insert(name.clone(), value);
                Ok(true)
            },

            expr => {
                self.eval(Expr::Binop(Box::new(expr), Binop::Eql, Box::new(value)))
                    .and_then(|result| result.truthiness())
            },
        }
    }

    fn eval(&self, expr: Expr) -> EvalResult<Expr> {
        if expr.is_self_evaluating() { return Ok(expr); }

        match expr {
            Expr::Not(expr) => Ok({
                let expr = try!(self.eval(*expr));
                let truth_value = try!(expr.truthiness());
                if truth_value {
                    Expr::lit_false()
                } else {
                    Expr::lit_true()
                }
            }),

            Expr::List(exprs) => Ok({
                let mut list = Vec::new();
                for expr in exprs {
                    list.push(try!(self.eval(expr)));
                }
                Expr::List(list)
            }),

            Expr::Var(v) => match self.env.get(&v) {
                Some(value) => Ok(value.clone()),
                None => Err(RuntimeError::NoSuchBinding(v)),
            },

            Expr::Binop(lhs, op, rhs) => {
                op.apply(try!(self.eval(*lhs)), try!(self.eval(*rhs)))
            },

            other => Err(RuntimeError::IrreducibleExpr(other)),
        }
    }

    fn exec(&mut self, knot: Knot, args: Vec<Expr>) -> EvalResult<()> {
        let wanted_count = knot.args.len();
        let got_count = args.len();
        if wanted_count != got_count {
            return Err({
                RuntimeError::WrongNumberOfArgs(wanted_count, got_count)
            });
        }

        // TODO: Insert module globals

        for (wanted, got) in knot.args.into_iter().zip(args.into_iter()) {
            try!(self.bind(wanted, got));
        }

        self.instructions = knot.body.into();

        Ok(())
    }

    fn hcf(&mut self, err: RuntimeError) {
        self.state = RunState::OnFire(err);
    }
}

impl Expr {
    fn is_self_evaluating(&self) -> bool {
        match self {
            &Expr::Actor(_) => true,
            &Expr::Atom(_) => true,
            &Expr::Int(_) => true,
            &Expr::Str(_) => true,
            &Expr::List(ref contents) => {
                contents.iter().all(|expr| expr.is_self_evaluating())
            },
            _ => false,
        }
    }

    fn truthiness(self) -> EvalResult<bool> {
        match self {
            Expr::Int(0) => Ok(false),
            Expr::Int(_) => Ok(true),
            _ => Ok(true),
        }
    }

    fn to_string(self) -> String {
        match self {
            Expr::Str(s) => s,
            Expr::Int(n) => format!("{}", n),
            Expr::Atom(a) => a,
            _ => unimplemented!(),
        }
    }

    fn from_bool(b: bool) -> Self {
        if b { Expr::Int(1) } else { Expr::Int(0) }
    }
}

impl Binop {
    fn apply(self, lhs: Expr, rhs: Expr) -> EvalResult<Expr> {
        use ast::Binop::*;
        use ast::Expr::*;

        match self {
            Add => match (lhs, rhs) {
                (Int(a), Int(b)) => Ok(Int(a + b)),
                (Str(mut s), z) => Ok({ s.push_str(&z.to_string()); Str(s) }),
                (a, b) => Err(RuntimeError::IllegalBinop(a, Add, b)),
            },

            Sub => match (lhs, rhs) {
                (Int(a), Int(b)) => Ok(Int(a - b)),
                (a, b) => Err(RuntimeError::IllegalBinop(a, Sub, b)),
            },

            Div => match (lhs, rhs) {
                (a, Int(0)) => Err(RuntimeError::DivideByZero(a)),
                (Int(a), Int(b)) => Ok(Int(a / b)),
                (a, b) => Err(RuntimeError::IllegalBinop(a, Div, b)),
            },

            Mul => match (lhs, rhs) {
                (Int(a), Int(b)) => Ok(Int(a * b)),
                (a, b) => Err(RuntimeError::IllegalBinop(a, Mul, b)),
            },

            Roll => match (lhs, rhs) {
                _ => unimplemented!(),
            },

            Eql => Ok(Expr::from_bool(match (lhs, rhs) {
                (Actor(a), Actor(b)) => a == b,
                (Int(a), Int(b)) => a == b,
                (Str(a), Str(b)) => a == b,
                (Atom(a), Atom(b)) => a == b,
                (List(a), List(b)) => a.len() == b.len() && {
                    let mut flag = true;
                    for (a, b) in a.into_iter().zip(b.into_iter()) {
                        let check = try!(Eql.apply(a, b));
                        let test = try!(check.truthiness());
                        if !test { flag = false; break; }
                    }
                    flag
                },

                _ => false,
            })),
        }
    }
}

impl ActorID {
    fn bump(&mut self) -> Self {
        let old = self.clone();
        self.0 += 1;
        old
    }
}

impl From<RuntimeError> for String {
    fn from(err: RuntimeError) -> Self {
        match err {
            RuntimeError::Unimplemented(stmt) => {
                format!("Feature not yet implemented: {:?}", stmt)
            },

            RuntimeError::NoSuchModule => {
                format!("Can't find the module <anonymous>")
            },

            RuntimeError::IrreducibleExpr(expr) => {
                format!("This expression can't be evaluated: {:?}", expr)
            },

            RuntimeError::NoSuchBinding(name) => {
                format!("The variable {:?} couldn't be found", name)
            },

            RuntimeError::NoSuchKnot(name) => {
                format!("Can't find the knot {:?}", name)
            },

            RuntimeError::IllegalLvalue(v) => {
                format!("Can't assign a value to {:?}", v)
            },

            RuntimeError::WrongNumberOfArgs(wanted, got) => {
                format!("Wanted {} args, but found {}", wanted, got)
            },

            RuntimeError::IllegalBinop(lhs, op, rhs) => {
                format!("Can't evaluate {:?} {:?} {:?}", lhs, op, rhs)
            },

            RuntimeError::DivideByZero(expr) => {
                format!("Can't divide {:?} by zero", expr)
            },
        }
    }
}

#[test]
fn compile_example() {
    let src = r#"
    == start
    -> test(1, 2)

    == test(A, B)
    > Value of A:
    trace A
    > Value of B:
    trace B

    trap
    | #bye from Self
        -> test2
    ;;

    Self <- #bye
    disarm 'nukes

    == test2
    > Doing great
    "#;

    let start = Label::Explicit("start".to_owned());

    let mut evaluator = Evaluator::new(100.0);
    evaluator.compile("main", src).expect("Compile error");
    evaluator.spawn(start, vec![]).expect("Spawn error");
    let result = evaluator.dispatch(1.0);
    match result {
        RunState::Idling => (),
        other => panic!("{:?}", other),
    }
}
