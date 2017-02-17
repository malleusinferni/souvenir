use std::collections::HashMap;

use ast;
use ir;

use ast::rewrite::Counter;

use driver::Try;

impl ast::pass::DesugaredProgram {
    pub fn translate(self) -> Try<ir::Program> {
        fn maketemp(id: u32) -> String {
            format!("TEMP%{:X}", id)
        }

        let mut builder = Builder {
            blocks: Vec::with_capacity(self.count_blocks()),

            pc: 0,
            bindings: Vec::new(),
            const_str: HashMap::new(),
            const_atom: HashMap::new(),
            next_var: Counter(0, ir::Var),
            next_tmp: Counter(0, maketemp),

            preludes: HashMap::new(),
            scenes: HashMap::new(),
            labels: HashMap::new(),
        };

        for scene in self.scenes {
            let label = builder.create_block()?;
            let name = scene.name.qualified()?;
            builder.scenes.insert(name, label);
        }

        ice!("TODO: Finish implementing")
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

    pc: usize,
    bindings: Vec<(String, ir::Var)>,
    const_str: HashMap<String, ir::Var>,
    const_atom: HashMap<String, ir::Var>,
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

            _ => ice!("Tried to modify a completed block"),
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
        self.emit(ir::Op::Let(var, value));
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

    fn intern_str(&mut self, t: ast::Str) -> Try<ir::Var> {
        ice!("Unimplemented")
    }

    fn intern_atom(&mut self, t: ast::Atom) -> Try<ir::Var> {
        ice!("Unimplemented")
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
                self.current()?.exit(ir::Exit::Recur(scene.with_argv(argv)))
            },

            ast::Stmt::Return { result } => {
                self.current()?.exit(ir::Exit::Return(result))
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
            ast::Expr::Str(s) => self.intern_str(s),

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
                ice!("Unimplemented")
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

            ast::Expr::PidZero => ice!("Unimplemented"),

            ast::Expr::Infinity => ice!("Unimplemented"),

            ast::Expr::Arg(_) => ice!("Unimplemented"),
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

    fn tr_lambda(&mut self, t: ast::TrapLambda) -> Try<ir::TrapRef> {
        let env = {
            let vars = t.captures.iter().cloned().map(|id| {
                ast::Expr::Id(id)
            }).collect::<Vec<_>>();

            self.tr_expr(ast::Expr::List(vars))?
        };

        let label = self.tr_label(t.label)?;

        Ok(ir::TrapRef {
            label: label,
            env: env,
        })
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
