use std::collections::HashMap;

use ast;
use ir;

use driver::Try;

enum Block {
    Partial(ir::BlockInfo, Vec<ir::Op>),
    Complete(ir::Block),
}

struct Builder {
    blocks: Vec<Block>,

    pc: usize,
    bindings: Vec<(String, ir::Var)>,
    const_str: HashMap<String, ir::Var>,
    const_int: HashMap<i32, ir::Var>,
    const_atom: HashMap<String, ir::Var>,
    next_temp_id: u32,

    preludes: HashMap<ast::Modpath, ir::Label>,
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
    fn create_block(&mut self, r: u32) -> Try<ir::Label> {
        let info = ir::BlockInfo {
            id: self.blocks.len() as u32,
            first_reg: r,
            flags_needed: 0,
        };

        self.blocks.push(Block::Partial(info, vec![]));
        Ok(ir::Label(info.id))
    }

    fn emit(&mut self, op: ir::Op) -> Try<()> {
        self.current()?.push(op)
    }

    fn assign(&mut self, name: &str, value: ir::Rvalue) -> Try<ir::Var> {
        let id = self.bindings.len() as u32;
        let reg = ir::Var(id);
        self.bindings.push((name.to_owned(), reg));
        self.emit(ir::Op::Let(reg, value));
        Ok(reg)
    }

    fn assign_temp(&mut self, value: ir::Rvalue) -> Try<ir::Var> {
        let id = self.next_temp_id;
        self.next_temp_id += 1;
        self.assign(&format!("TEMP#{:X}", id), value)
    }

    fn eval(&self, name: &str) -> Try<ir::Var> {
        for &(ref key, val) in self.bindings.iter().rev() {
            if key == name { return Ok(val); }
        }

        ice!("Failed to look up variable {}", name);
    }

    fn set(&mut self, value: ir::Tvalue) -> Try<ir::Flag> {
        let counter = &mut self.current()?.info().flags_needed;
        let flag = ir::Flag(*counter);
        *counter += 1;
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

    fn intern_int(&mut self, t: i32) -> Try<ir::Var> {
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

                let first_reg = self.current()?.info().first_reg;
                let succ = self.create_block(first_reg)?;
                let fail = self.create_block(first_reg)?;
                let next = self.create_block(first_reg)?;

                self.current()?
                    .exit(ir::Exit::IfThenElse(test, succ, fail))?;

                self.jump(succ)?;
                for stmt in success.0.into_iter() {
                    self.tr_stmt(stmt)?;
                }
                self.current()?.exit(ir::Exit::Goto(next))?;

                self.jump(fail)?;
                for stmt in success.0.into_iter() {
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

            ast::Stmt::Listen { name, arms } => {
                let target = self.tr_label(target)?;
                self.emit(ir::Op::Listen(target))
            },

            ast::Stmt::SendMsg { message, target } => {
                let message = self.tr_expr(message)?;
                let target = self.tr_expr(target)?;
                self.emit(ir::Op::SendMsg(target, message))
            },

            ast::Stmt::Wait { value } => {
                let value = self.tr_expr(value)?;
                self.emit(ir::Op::Wait(value))
            },
        }
    }

    fn tr_expr(&mut self, t: ast::Expr) -> Try<ir::Var> {
        match t {
            ast::Expr::Atom(a) => self.intern_atom(a),
            ast::Expr::Str(s) => self.intern_str(s),
            ast::Expr::Int(i) => self.intern_int(i),

            ast::Expr::Id(id) => self.eval(&id.name),

            ast::Expr::List(items) => {
                let mut vars = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    vars.push(self.tr_expr(item)?);
                }
                self.assign_temp(ir::Rvalue::ListOf(vars))
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

            ast::Expr::Nth(list, index) => {
                let index = self.intern_int(index as i32)?;
                let list = self.tr_expr(*list)?;
                self.assign_temp(ir::Rvalue::Nth(list, index))
            },

            ast::Expr::Spawn(call) => {
                ice!("Unimplemented")
            },

            ast::Expr::PidOfSelf => {
                self.assign_temp(ir::Rvalue::PidOfSelf)
            },

            ast::Expr::Splice(items) => {
                let mut vars = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    vars.push(self.tr_expr(item)?);
                }
                self.assign_temp(ir::Rvalue::Splice(vars))
            },

            ast::Expr::PidZero => ice!("Unimplemented"),

            ast::Expr::Infinity => ice!("Unimplemented"),

            ast::Expr::Arg => ice!("Unimplemented"),
        }
    }

    fn tr_cond(&mut self, t: ast::Cond) -> Try<ir::Flag> {
        match t {
            ast::Cond::True => self.set(ir::Tvalue::True),
            ast::Cond::False => self.set(ir::Tvalue::False),
            ast::Cond::LastResort => ice!("Un-desugared LastResort"),

            ast::Cond::HasLength(list, len) => {
                let list = self.tr_expr(list)?;
                let len = self.intern_int(len as i32)?;
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
        ice!("Unimplemented")
    }
}
