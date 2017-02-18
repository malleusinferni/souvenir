use std::collections::HashMap;

use string_interner::StringInterner;

use ast;
use ir;

use ast::pass::DesugaredProgram;
use ast::rewrite::Counter;

use driver::Try;

impl DesugaredProgram {
    pub fn translate(self) -> Try<ir::Program> {
        fn maketemp(id: u32) -> String {
            format!("TEMP%{:X}", id)
        }

        let mut builder = Builder {
            blocks: Vec::with_capacity(self.count_blocks()),
            str_table: StringInterner::new(),
            atom_table: StringInterner::new(),

            pc: 0,
            bindings: Vec::new(),
            envs: HashMap::new(),
            next_var: Counter(0, ir::Var),
            next_tmp: Counter(0, maketemp),

            preludes: HashMap::new(),
            scenes: HashMap::new(),
            labels: HashMap::new(),
        };

        // Prelude entry point must be block 0
        let _ = builder.create_block()?;

        for scene in self.scenes.iter() {
            let label = builder.create_block()?;
            let name = scene.name.qualified()?;
            builder.scenes.insert(name, label);
        }

        for lambda in self.lambdas.iter() {
            let label = builder.create_block()?;
            let name = lambda.label.qualified()?;
            builder.labels.insert(name, label);
        }

        // Setup done

        builder.tr_program(self)
    }

    fn count_blocks(&self) -> usize {
        1024 // TODO: Actually count them
    }
}

enum Block {
    Partial(ir::BlockInfo, Vec<ir::Op>),
    Complete(ir::Block),
}

enum Func {
    Scene(ast::QfdSceneName),
    Lambda(ast::QfdLabel),
}

struct Builder {
    blocks: Vec<Block>,
    str_table: StringInterner<ir::StrId>,
    atom_table: StringInterner<ir::AtomId>,

    pc: usize,
    bindings: Vec<(String, ir::Var)>,
    envs: HashMap<ast::Modpath, Vec<(String, ir::Rvalue)>>,

    next_var: Counter<ir::Var>,
    next_tmp: Counter<String>,

    preludes: HashMap<ast::Modpath, ir::Label>,
    scenes: HashMap<ast::QfdSceneName, ir::Label>,
    labels: HashMap<ast::QfdLabel, ir::Label>,
}

impl Block {
    fn exit(&mut self, exit: ir::Exit) -> Try<()> {
        let (info, code) = match self {
            &mut Block::Partial(ref info, ref code) => {
                (info.clone(), code.clone())
            },

            _ => ice!("Tried to exit from a completed block"),
        };

        *self = Block::Complete(ir::Block {
            info: info,
            ops: code,
            exit: exit,
        });

        Ok(())
    }

    fn push(&mut self, op: ir::Op) -> Try<()> {
        match self {
            &mut Block::Partial(_, ref mut ops) => {
                ops.push(op);
                Ok(())
            },

            &mut Block::Complete(ref mut block) => {
                ice!("Tried to modify a completed block\n{:?}\n{:?}", block, op)
            },
        }
    }

    fn info(&mut self) -> &mut ir::BlockInfo {
        match self {
            &mut Block::Partial(ref mut info, _) => info,
            &mut Block::Complete(ref mut ir_block) => &mut ir_block.info,
        }
    }
}

impl Builder {
    fn create_block(&mut self) -> Try<ir::Label> {
        let id = self.blocks.len() as u32;

        let info = ir::BlockInfo {
            id: id,
            flags_needed: 0,
        };

        self.blocks.push(Block::Partial(info, vec![]));
        Ok(ir::Label(id))
    }

    fn emit(&mut self, op: ir::Op) -> Try<()> {
        self.current()?.push(op)
    }

    fn assign(&mut self, name: &str, value: ir::Rvalue) -> Try<ir::Var> {
        let var = self.next_var.next();
        self.bindings.push((name.to_owned(), var));
        self.emit(ir::Op::Let(var, value))?;
        Ok(var)
    }

    fn assign_temp(&mut self, value: ir::Rvalue) -> Try<ir::Var> {
        let name = self.next_tmp.next();
        self.assign(&name, value)
    }

    fn eval(&self, name: &str) -> Try<ir::Var> {
        for &(ref key, val) in self.bindings.iter().rev() {
            if key == name { return Ok(val); }
        }

        ice!("Failed to look up variable {}", name);
    }

    fn set(&mut self, value: ir::Tvalue) -> Try<ir::Flag> {
        let flag = ir::Flag({
            let counter = &mut self.current()?.info().flags_needed;
            let val = *counter;
            *counter += 1;
            val
        });

        self.emit(ir::Op::Set(flag, value))?;
        Ok(flag)
    }

    fn jump(&mut self, label: ir::Label) -> Try<()> {
        self.pc = label.0 as usize;
        Ok(())
    }

    fn current(&mut self) -> Try<&mut Block> {
        match self.blocks.get_mut(self.pc) {
            Some(block) => Ok(block),
            None => ice!("Block index out of bounds"),
        }
    }

    fn capture_env(&mut self, modpath: ast::Modpath) -> Try<()> {
        let named_vars = {
            self.bindings.iter().filter_map(|&(ref name, _)| {
                if name.contains("TEMP%") {
                    None
                } else {
                    Some(name.clone())
                }
            }).collect::<Vec<String>>()
        };

        let env_items = {
            self.tr_expr(ast::Expr::List({
                named_vars.iter().map(|name| ast::Expr::Id(ast::Ident {
                    name: name.clone(),
                })).collect()
            }))?
        };

        let env_id = ir::Env(self.envs.len() as u32);
        self.emit(ir::Op::Export(env_id, env_items))?;

        let mappings = named_vars.into_iter().enumerate().map(|(i, name)| {
            (name, ir::Rvalue::LoadEnv(i as u32))
        }).collect();

        self.envs.insert(modpath, mappings);
        self.bindings.clear();

        Ok(())
    }

    fn intern_str(&mut self, t: &str) -> Try<ir::Var> {
        let s = ir::ConstRef::Str(self.str_table.get_or_intern(t));
        self.assign_temp(ir::Rvalue::Const(s))
    }

    fn intern_atom(&mut self, t: ast::Atom) -> Try<ir::Var> {
        let ast::Atom::User(a) = t;
        let a = ir::ConstRef::Atom(self.atom_table.get_or_intern(a));
        self.assign_temp(ir::Rvalue::Const(a))
    }

    fn tr_program(mut self, t: DesugaredProgram) -> Try<ir::Program> {
        self.jump(ir::Label(0))?;
        for (modpath, body) in t.preludes {
            self.tr_block(body)?;
            self.capture_env(modpath)?;
        }
        self.current()?.exit(ir::Exit::EndProcess)?;

        for scene in t.scenes {
            self.tr_scene(scene)?;
        }

        for lambda in t.lambdas {
            self.tr_lambda(lambda)?;
        }

        Ok(ir::Program {
            blocks: self.blocks.into_iter().map(|block| match block {
                Block::Complete(block) => Ok(block),
                Block::Partial(info, block) => {
                    ice!("Incomplete block: {:?}", block)
                },
            }).collect::<Try<_>>()?,
            str_table: self.str_table,
            atom_table: self.atom_table,
        })
    }

    fn tr_scene(&mut self, t: ast::Scene) -> Try<()> {
        let env = {
            let qfd = t.name.qualified()?;
            match self.envs.get(&qfd.in_module) {
                Some(env) => env.clone(),
                None => ice!("Missing env for {}", qfd),
            }
        };

        let label = self.tr_scene_name(t.name)?;
        self.jump(label)?;

        for (name, value) in env.into_iter() {
            self.assign(&name, value)?;
        }

        for (i, arg) in t.args.into_iter().enumerate() {
            if let Some(ast::Ident { name }) = arg {
                self.assign(&name, ir::Rvalue::Arg(i as u32))?;
            }
        }

        self.tr_block(t.body)?;

        self.current()?.exit(ir::Exit::EndProcess)?;
        self.bindings.clear();

        Ok(())
    }

    fn tr_lambda(&mut self, t: ast::TrapLambda) -> Try<()> {
        let label = self.tr_label(t.label)?;
        self.jump(label)?;

        // NOTE: Environment is built dynamically by Stmt::Arm
        for (i, ast::Ident { name }) in t.captures.into_iter().enumerate() {
            self.assign(&name, ir::Rvalue::LoadEnv(i as u32))?;
        }

        self.tr_block(t.body)?;
        self.current()?.exit(ir::Exit::Return(false))?;

        Ok(())
    }

    fn tr_block(&mut self, t: ast::Block) -> Try<()> {
        for stmt in t.0 {
            self.tr_stmt(stmt)?;
        }
        Ok(())
    }

    fn tr_stmt(&mut self, t: ast::Stmt) -> Try<()> {
        match t {
            ast::Stmt::Empty => Ok(()),

            ast::Stmt::Disarm { target } => {
                let target = self.tr_label(target)?;
                self.emit(ir::Op::Disarm(target))
            },

            ast::Stmt::Discard { value } => {
                let _ = self.tr_expr(value)?;
                Ok(())
            },

            ast::Stmt::If { test, success, failure } => {
                let test = self.tr_cond(test)?;

                let succ = self.create_block()?;
                let fail = self.create_block()?;
                let next = self.create_block()?;

                self.current()?
                    .exit(ir::Exit::IfThenElse(test, succ, fail))?;

                self.jump(succ)?;
                for stmt in success.0.into_iter() {
                    self.tr_stmt(stmt)?;
                }
                self.current()?.exit(ir::Exit::Goto(next))?;

                self.jump(fail)?;
                for stmt in failure.0.into_iter() {
                    self.tr_stmt(stmt)?;
                }
                self.current()?.exit(ir::Exit::Goto(next))?;

                self.jump(next)
            },

            ast::Stmt::Let { name: ast::Ident { name }, value } => {
                let value = self.tr_expr(value)?;
                self.assign(&name, ir::Rvalue::Var(value))?;
                Ok(())
            },

            ast::Stmt::Arm { target, with_env, blocking } => {
                let trap_ref = ir::TrapRef {
                    label: self.tr_label(target)?,
                    env: self.tr_expr(with_env)?,
                };

                if blocking {
                    self.emit(ir::Op::Listen(trap_ref))
                } else {
                    self.emit(ir::Op::Arm(trap_ref))
                }
            },

            ast::Stmt::Recur { target } => {
                let ast::Call(scene, args) = target;
                let scene = self.tr_scene_name(scene)?;
                let argv = self.tr_expr(ast::Expr::List(args))?;
                self.current()?.exit(ir::Exit::Recur(scene.with_argv(argv)))?;

                let unreachable = self.create_block()?;
                self.jump(unreachable)
            },

            ast::Stmt::Return { result } => {
                self.current()?.exit(ir::Exit::Return(result))?;

                let unreachable = self.create_block()?;
                self.jump(unreachable)
            },

            ast::Stmt::Say { message } => {
                let message = self.tr_expr(message)?;
                self.emit(ir::Op::Say(message))
            },

            ast::Stmt::SendMsg { message, target } => {
                let message = self.tr_expr(message)?;
                let target = self.tr_expr(target)?;
                self.emit(ir::Op::SendMsg(target, message))
            },

            ast::Stmt::Trace { value } => {
                let value = self.tr_expr(value)?;
                self.emit(ir::Op::Trace(value))
            },

            ast::Stmt::Wait { value } => {
                let value = self.tr_expr(value)?;
                self.emit(ir::Op::Wait(value))
            },

            ast::Stmt::Listen { .. }
            | ast::Stmt::Match { .. }
            | ast::Stmt::Naked { .. }
            | ast::Stmt::Trap { .. }
            | ast::Stmt::Weave { .. } => {
                ice!("Syntax must be desugared before translation")
            },
        }
    }

    fn tr_expr(&mut self, t: ast::Expr) -> Try<ir::Var> {
        match t {
            ast::Expr::Atom(a) => self.intern_atom(a),

            ast::Expr::Str(s) => match s {
                ast::Str::Plain(s) => self.intern_str(&s),
            },

            ast::Expr::Int(i) => {
                self.assign_temp(ir::Rvalue::Int(i))
            },

            ast::Expr::Bool(b) => {
                let flag = self.tr_cond(*b)?;
                self.assign_temp(ir::Rvalue::FromBool(flag))
            },

            ast::Expr::Id(id) => self.eval(&id.name),

            ast::Expr::List(items) => {
                let len = items.len() as u32;
                let list_ptr = self.assign_temp(ir::Rvalue::Alloc(len))?;
                for (i, item) in items.into_iter().enumerate() {
                    let item = self.tr_expr(item)?;
                    let addr = list_ptr.at_offset(i as u32);
                    self.emit(ir::Op::Store(item, addr))?;
                }
                Ok(list_ptr)
            },

            ast::Expr::Op(op, mut operands) => {
                operands.reverse();

                let mut lhs = match operands.pop() {
                    Some(ok) => self.tr_expr(ok)?,
                    None => ice!("Zero-operand expression"),
                };

                while let Some(rhs) = operands.pop() {
                    let rhs = self.tr_expr(rhs)?;
                    lhs = self.assign_temp(match &op {
                        &ast::Op::Add => ir::Rvalue::Add(lhs, rhs),
                        &ast::Op::Sub => ir::Rvalue::Sub(lhs, rhs),
                        &ast::Op::Div => ir::Rvalue::Div(lhs, rhs),
                        &ast::Op::Mul => ir::Rvalue::Mul(lhs, rhs),
                        &ast::Op::Roll => ir::Rvalue::Roll(lhs, rhs),
                    })?;
                }

                Ok(lhs)
            },

            ast::Expr::MenuChoice(choices) => {
                let list_ptr = self.tr_expr(ast::Expr::List(choices))?;
                self.assign_temp(ir::Rvalue::MenuChoice(list_ptr))
            },

            ast::Expr::Nth(list, index) => {
                let list = self.tr_expr(*list)?;
                self.assign_temp(ir::Rvalue::Load(list.at_offset(index)))
            },

            ast::Expr::Spawn(call) => {
                let ast::Call(scene, args) = call;
                let scene = self.tr_scene_name(scene)?;
                let argv = self.tr_expr(ast::Expr::List(args))?;
                self.assign_temp(ir::Rvalue::Spawn(scene.with_argv(argv)))
            },

            ast::Expr::PidOfSelf => {
                self.assign_temp(ir::Rvalue::PidOfSelf)
            },

            ast::Expr::Splice(items) => {
                let items = items.into_iter().map(|item| {
                    self.tr_expr(item)
                }).collect::<Try<_>>()?;

                self.assign_temp(ir::Rvalue::Splice(items))
            },

            ast::Expr::Arg(n) => {
                self.assign_temp(ir::Rvalue::Arg(n))
            },

            ast::Expr::PidZero => ice!("Unimplemented: PID zero"),

            ast::Expr::Infinity => ice!("Unimplemented: Infinity"),
        }
    }

    fn tr_cond(&mut self, t: ast::Cond) -> Try<ir::Flag> {
        match t {
            ast::Cond::True => self.set(ir::Tvalue::True),
            ast::Cond::False => self.set(ir::Tvalue::False),
            ast::Cond::LastResort => ice!("Un-desugared LastResort"),

            ast::Cond::HasLength(list, len) => {
                let list = self.tr_expr(list)?;
                let len = self.tr_expr(ast::Expr::Int(len as i32))?;
                self.set(ir::Tvalue::HasLen(list, len))
            },

            ast::Cond::Compare(rel, lhs, rhs) => {
                let lhs = self.tr_expr(lhs)?;
                let rhs = self.tr_expr(rhs)?;
                match rel {
                    ast::BoolOp::Eql => {
                        self.set(ir::Tvalue::Eql(lhs, rhs))
                    },

                    ast::BoolOp::Gt => {
                        self.set(ir::Tvalue::Gt(lhs, rhs))
                    },

                    ast::BoolOp::Lt => {
                        self.set(ir::Tvalue::Lt(lhs, rhs))
                    },

                    ast::BoolOp::Gte => {
                        self.set(ir::Tvalue::Gte(lhs, rhs))
                    },

                    ast::BoolOp::Lte => {
                        self.set(ir::Tvalue::Lte(lhs, rhs))
                    },
                }
            },

            ast::Cond::And(conds) => {
                let mut flags = Vec::with_capacity(conds.len());
                for cond in conds.into_iter() {
                    flags.push(self.tr_cond(cond)?);
                }
                self.set(ir::Tvalue::And(flags))
            },

            ast::Cond::Or(conds) => {
                let mut flags = Vec::with_capacity(conds.len());
                for cond in conds.into_iter() {
                    flags.push(self.tr_cond(cond)?);
                }
                self.set(ir::Tvalue::Or(flags))
            },

            ast::Cond::Not(cond) => {
                let flag = self.tr_cond(*cond)?;
                self.set(ir::Tvalue::Not(flag))
            },
        }
    }

    fn tr_label(&mut self, t: ast::Label) -> Try<ir::Label> {
        let t = t.qualified()?;
        match self.labels.get(&t) {
            Some(&label) => Ok(label),
            None => ice!("Label {} has no entry point", t),
        }
    }

    fn tr_scene_name(&mut self, t: ast::SceneName) -> Try<ir::Label> {
        let t = t.qualified()?;
        match self.scenes.get(&t) {
            Some(&label) => Ok(label),
            None => ice!("Scene name {} has no entry point", t),
        }
    }
}
