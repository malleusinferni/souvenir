//! Defines an AST evaluator.

use std::collections::{HashMap, VecDeque};

use ast::*;

pub struct Process {
    id: ActorID,
    state: RunState,
    env: Env,
    traps: VecDeque<(Env, Trap)>,
    outbuf: VecDeque<String>,
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
    IllegalConversion(Expr, &'static str),
}

pub type EvalResult<T> = Result<T, RuntimeError>;

pub struct Message {
    src: ActorID,
    dst: ActorID,
    body: Expr,
}

pub struct Evaluator {
    modules: HashMap<String, (Env, Module)>,
    processes: Vec<Process>,
    messages: VecDeque<Message>,
    stdout: VecDeque<String>,
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
            stdout: VecDeque::new(),
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

        if module.globals.len() > 0 {
            return Err(format!("Module globals not yet supported"));
        }

        self.modules.insert(name.to_owned(), (Env::new(), module));

        Ok(())
    }

    pub fn spawn(&mut self, label: Label, args: Vec<Expr>) -> EvalResult<ActorID> {
        let id = self.next_actor_id.bump();
        let mut process = Process::new(id.clone());

        let (knot, env) = try!(self.find_knot(label));
        try!(process.exec(knot, env, args));
        self.processes.push(process);

        Ok(id)
    }

    pub fn choose(&mut self, i: usize) {
        for process in self.processes.iter_mut() {
            if process.id.0 != 0 { continue; }

            let options = match process.state.clone() {
                RunState::WaitingForInput(options) => options,
                _ => return,
            };

            let (_, mut statements) = match options.get(i) {
                Some(s) => s.clone().into(),
                None => return,
            };

            statements.extend(process.instructions.drain(..));
            process.instructions = statements.into();
            process.state = RunState::Running;
        }
    }

    pub fn with_stdout<F: Fn(String)>(&mut self, action: F) {
        while let Some(s) = self.stdout.pop_front() {
            action(s);
        }
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

                    RunState::Sleeping(mut n) => {
                        n -= sleep_step;
                        process.state = if n <= 0.0 {
                            RunState::Running
                        } else {
                            RunState::Sleeping(n)
                        };
                    },

                    RunState::Running => {
                        self.run_once(&mut process)
                            .unwrap_or_else(|e| process.hcf(e));
                    },

                    _ => (),
                }

                if process.id.0 == 0 {
                    main_process_state = Some(process.state.clone());

                    match process.outbuf.pop_front() {
                        Some(s) => self.stdout.push_back(s),
                        None => (),
                    }
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

            Stmt::Let(name, value) => {
                if try!(process.bind(name.clone(), value)) {
                    ()
                } else {
                    panic!("Binding failed");
                }
            },

            Stmt::LetSpawn(name, label, args) => {
                let mut arg_values = Vec::with_capacity(args.len());
                for arg in args {
                    arg_values.push(try!(process.eval(arg)));
                }

                // Okay, here's the fucked up part...
                let child_id = try!(self.spawn(label, arg_values));
                try!(process.bind(name, Expr::Actor(child_id)));
            },

            Stmt::Trace(expr) => {
                let output = try!(process.eval(expr)).to_string();
                process.outbuf.push_back(output);
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

                let (knot, env) = try!(self.find_knot(label));
                try!(process.exec(knot, env, arg_values));
                // It SHOULD be that easy... right?
            },

            Stmt::Wait(expr) => {
                let time = try!(process.eval(expr).and_then(|n| n.to_int()));
                process.state = RunState::Sleeping(time as f32 / 100.0);
            },

            other_stmt => {
                process.state = RunState::OnFire({
                    RuntimeError::Unimplemented(other_stmt)
                });
            },
        }

        Ok(())
    }

    fn find_knot(&self, label: Label) -> EvalResult<(Knot, Env)> {
        // TODO: Namespaced labels

        let &(ref env, ref module) = match self.modules.values().next() {
            Some(m) => m,
            None => return Err(RuntimeError::NoSuchModule),
        };

        for knot in module.knots.iter() {
            if knot.name == label {
                return Ok((knot.clone(), env.clone()));
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
        Process {
            id: id,
            env: HashMap::new(),
            state: RunState::Running,
            traps: VecDeque::new(),
            outbuf: VecDeque::new(),
            instructions: VecDeque::new(),
        }
    }

    fn reset_env(&mut self) {
        self.env = HashMap::new();
        self.env.insert("Self".to_owned(), Expr::Actor(self.id.clone()));
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

    fn exec(&mut self, knot: Knot, env: Env, args: Vec<Expr>) -> EvalResult<()> {
        let wanted_count = knot.args.len();
        let got_count = args.len();
        if wanted_count != got_count {
            return Err({
                RuntimeError::WrongNumberOfArgs(wanted_count, got_count)
            });
        }

        self.reset_env();
        self.env.extend(env.into_iter());

        for (wanted, got) in knot.args.into_iter().zip(args.into_iter()) {
            try!(self.bind(wanted, got));
        }

        self.instructions = knot.body.into();
        self.state = RunState::Running;

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

    fn to_int(self) -> EvalResult<i32> {
        match self {
            Expr::Int(n) => Ok(n),
            other => Err(RuntimeError::IllegalConversion(other, "int"))
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

            RuntimeError::IllegalConversion(expr, wanted) => {
                format!("Can't convert {:?} to {}", expr, wanted)
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

    let C = #oh_no
    trace C

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
