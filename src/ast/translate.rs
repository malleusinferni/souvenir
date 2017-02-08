use std::collections::HashMap;

use ast;
use ir;

use driver::{Try, ErrCtx, BuildErr};

impl ast::Program {
    pub fn to_ir(self) -> Try<ir::Program> {
        self.check_names()?;
        self.check_prelude_restrictions()?;
        self.check_variable_definitions()?;

        let mut tr = Translator {
            program: ir::Program {
                preludes: vec![],
                scenes: vec![],
            },

            context: ErrCtx::NoContext,
            env: vec![],
        };

        tr.translate(self)
    }
}

type Peek<'i, T> = ::std::iter::Peekable<::std::slice::Iter<'i, T>>;

struct Translator {
    program: ir::Program,
    context: ErrCtx,
    env: Vec<HashMap<String, ()>>,
}

impl Translator {
    fn translate(mut self, input: ast::Program) -> Try<ir::Program> {
        for (modpath, module) in input.modules.into_iter() {
            self.tr_module(module, modpath)?;
        }

        Ok(self.program)
    }

    fn enter(&mut self) -> Try<()> {
        self.env.push(HashMap::new());

        Ok(())
    }

    fn leave(&mut self) -> Try<()> {
        if self.env.pop().is_none() {
            ice!("Stack management failure")
        }

        Ok(())
    }

    fn is_defined(&mut self, name: &str) -> Try<bool> {
        for scope in self.env.iter().rev() {
            if scope.contains_key(name) { return Ok(true) }
        }

        Ok(false)
    }

    fn eval(&mut self, name: &str) -> Try<()> {
        if self.is_defined(name)? {
            Ok(())
        } else {
            ice!("Undefined variable: {}", name)
        }
    }

    fn assign(&mut self, name: &str) -> Try<()> {
        if let Some(scope) = self.env.iter_mut().last() {
            scope.insert(name.to_owned(), ());
            Ok(())
        } else {
            ice!("Assignment outside scope")
        }
    }

    fn tr_module(&mut self, t: ast::Module, p: ast::Modpath) -> Try<()> {
        let path = p;

        self.enter()?;

        let mut prelude = ir::Scope { body: vec![] };
        for stmt in t.globals.0.into_iter() {
            self.tr_global(stmt, &mut prelude)?;
        }

        for scene in t.scenes.into_iter() {
            self.tr_scene(scene, &path)?;
        }

        self.leave()?;

        self.program.preludes.push(prelude);

        Ok(())
    }

    fn tr_global(&mut self, t: ast::Stmt, p: &mut ir::Scope) -> Try<()> {
        let stmt = match t {
            ast::Stmt::Empty => return Ok(()),

            ast::Stmt::Discard { value } => ir::Stmt::Discard {
                value: self.tr_expr(value)?,
            },

            ast::Stmt::Let { value, name } => ir::Stmt::Let {
                value: self.tr_expr(value)?,
                dest: {
                    let name = match name {
                        ast::Ident::Var { name } => name,
                        ast::Ident::PidOfSelf => ice!("Assigned to Self"),
                    };

                    self.assign(&name)?;
                    ir::Var::Id(name)
                },
            },

            other => ice!("Not permitted: {}", other),
        };

        p.body.push(stmt);

        Ok(())
    }

    fn tr_scene(&mut self, t: ast::Scene, p: &ast::Modpath) -> Try<()> {
        let name = &t.name.name;

        if let Some(modpath) = t.name.in_module.as_ref() {
            ice!("Overqualified scene name {}", &t.name)
        }

        self.enter()?;

        let wanted = t.args.len() as u32;

        let mut scope: ir::Scope = vec![].into();

        for arg in t.args {
            scope.body.push(ir::Stmt::Let {
                value: ir::Expr::FetchArgument,
                dest: match arg {
                    ast::Ident::Var { name } => ir::Var::Id(name),

                    ast::Ident::PidOfSelf => {
                        ice!("Used Self as an argument name")
                    },
                },
            });
        }

        scope.body.extend(self.tr_block(t.body)?.body);

        self.leave()?;

        let prelude_id = self.program.preludes.len();

        self.program.scenes.push(ir::SceneDef {
            prelude_id: prelude_id,
            args_wanted: wanted,
            body: scope,
        });

        Ok(())
    }

    fn tr_block(&mut self, t: ast::Block) -> Try<ir::Scope> {
        self.enter()?;

        let ast::Block(block) = t;

        let mut scope = ir::Scope { body: vec![] };

        for stmt in block.into_iter() {
            if let &ast::Stmt::Empty = &stmt { continue; }
            scope.body.push(self.tr_stmt(stmt)?);
        }

        self.leave()?;

        Ok(scope)
    }

    fn tr_stmt(&mut self, t: ast::Stmt) -> Try<ir::Stmt> {
        let t = match t {
            ast::Stmt::Empty => {
                ice!("Forgot to skip a blank line")
            },

            ast::Stmt::Disarm { target } => ir::Stmt::Disarm {
                name: self.tr_label(target)?,
            },

            ast::Stmt::Discard { value } => ir::Stmt::Discard {
                value: self.tr_expr(value)?,
            },

            ast::Stmt::Let { value, name } => ir::Stmt::Let {
                value: self.tr_expr(value)?,
                dest: ir::Var::Id(match name {
                    ast::Ident::Var { name } => {
                        self.assign(&name)?;
                        name
                    },
                    _ => ice!("Invalid assign"),
                }),
            },

            ast::Stmt::Listen { name, arms } => ir::Stmt::Sugar {
                stmt: ir::SugarStmt::Listen {
                    label: self.tr_label(name)?,
                    arms: {
                        let mut tr_arms = Vec::with_capacity(arms.len());
                        for arm in arms.into_iter() {
                            tr_arms.push(ir::TrapArm {
                                pattern: self.tr_pat(arm.pattern)?,
                                sender: self.tr_pat(arm.origin)?,
                                guard: self.tr_cond(arm.guard)?,
                                body: self.tr_block(arm.body)?,
                            });
                        }

                        tr_arms
                    },
                },
            },

            ast::Stmt::Naked { .. } => {
                ice!("Forgot to reflow a print statement: {:?}", t)
            },

            ast::Stmt::Recur { target } => ir::Stmt::Recur {
                target: self.tr_call(target)?,
            },

            ast::Stmt::SendMsg { message, target } => {
                let target = ast::Expr::Id(target.clone());
                ir::Stmt::SendMsg {
                    target: self.tr_expr(target)?,
                    message: self.tr_expr(message)?,
                }
            },

            ast::Stmt::Trace { value } => ir::Stmt::Trace {
                value: self.tr_expr(value)?,
            },

            ast::Stmt::Wait { value } => ir::Stmt::Wait {
                value: self.tr_expr(value)?,
            },

            ast::Stmt::Weave { name, arms } => ir::Stmt::Sugar {
                stmt: ir::SugarStmt::Weave {
                    label: self.tr_label(name)?,
                    arms: {
                        let mut tr_arms = Vec::with_capacity(arms.len());
                        for arm in arms.into_iter() {
                            tr_arms.push(ir::WeaveArm {
                                guard: self.tr_cond(arm.guard)?,
                                message: self.tr_expr(arm.message)?,
                                body: self.tr_block(arm.body)?,
                            })
                        }

                        tr_arms
                    },
                },
            },

            ast::Stmt::Trap { name, arms } => {
                let mut tr_arms = Vec::with_capacity(arms.len());
                for t in arms {
                    tr_arms.push(ir::TrapArm {
                        pattern: self.tr_pat(t.pattern)?,
                        sender: self.tr_pat(t.origin)?,
                        guard: self.tr_cond(t.guard)?,
                        body: self.tr_block(t.body)?,
                    });
                }

                ir::Stmt::Sugar {
                    stmt: ir::SugarStmt::Trap {
                        label: self.tr_label(name)?,
                        arms: tr_arms,
                    },
                }
            },
        };

        Ok(t)
    }

    fn tr_expr(&mut self, t: ast::Expr) -> Try<ir::Expr> {
        let t = match t {
            ast::Expr::Id(id) => match id {
                ast::Ident::PidOfSelf => {
                    ir::Expr::PidOfSelf
                },

                ast::Ident::Var { name } => {
                    self.eval(&name)?;
                    ir::Expr::Var(ir::Var::Id(name))
                },
            },

            ast::Expr::Lit(lit) => {
                self.tr_literal(lit)?
            },

            ast::Expr::Str(_) => {
                unimplemented!()
            },

            ast::Expr::Op(_, _) => {
                ice!("Binops not implemented")
            },

            ast::Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.tr_expr(item)?);
                }
                ir::Expr::List(out)
            },

            ast::Expr::Spawn(call) => {
                ir::Expr::Spawn(self.tr_call(call)?)
            },
        };

        Ok(t)
    }

    fn tr_pat(&mut self, t: ast::Pat) -> Try<ir::Pat> {
        let t = match t {
            ast::Pat::Hole => ir::Pat::Hole,

            ast::Pat::Id(id) => match id {
                ast::Ident::PidOfSelf => ir::Pat::EqualTo({
                    ir::Expr::PidOfSelf
                }),

                ast::Ident::Var { name } => {
                    if self.is_defined(&name)? {
                        self.eval(&name)?;
                        ir::Pat::EqualTo({
                            ir::Expr::Var({
                                ir::Var::Id(name)
                            })
                        })
                    } else {
                        self.assign(&name)?;
                        ir::Pat::Assign(ir::Var::Id(name))
                    }
                },
            },

            ast::Pat::Lit(lit) => {
                ir::Pat::EqualTo(self.tr_literal(lit)?)
            },

            ast::Pat::List(items) => {
                let mut pats = Vec::with_capacity(items.len());
                for item in items.into_iter() {
                    pats.push(self.tr_pat(item)?);
                }
                ir::Pat::List(pats)
            },
        };

        Ok(t)
    }

    fn tr_label(&mut self, t: ast::Label) -> Try<ir::Label> {
        unimplemented!()
    }

    fn tr_literal(&mut self, t: ast::Lit) -> Try<ir::Expr> {
        let t = match t {
            ast::Lit::Atom(name) => {
                ir::Expr::Atom(ir::Atom::User(name.clone()))
            },

            ast::Lit::Int(n) => {
                ir::Expr::Int(n)
            },

            ast::Lit::InvalidInt(digits) => {
                ice!("Invalid int: {}", digits);
            },
        };

        Ok(t)
    }

    fn tr_cond(&mut self, t: ast::Cond) -> Try<ir::Cond> {
        Ok(match t {
            ast::Cond::True => ir::Cond::True,
            ast::Cond::False => ir::Cond::False,
            ast::Cond::LastResort => ir::Cond::LastResort,

            ast::Cond::Not(cond) => ir::Cond::Not(Box::new({
                self.tr_cond(*cond)?
            })),

            ast::Cond::Compare(ast::BoolOp::Eql, lhs, rhs) => {
                let lhs = self.tr_expr(lhs)?;
                let rhs = self.tr_expr(rhs)?;
                ir::Cond::Equals(lhs, rhs)
            },

            other => ice!("Not implemented: {}", other),
        })
    }

    fn tr_call(&mut self, t: ast::Call) -> Try<ir::Call> {
        let ast::Call(name, args) = t;

        let scene_id = unimplemented!();

        let mut argv = Vec::with_capacity(args.len());
        for arg in args {
            argv.push(self.tr_expr(arg)?);
        }

        Ok(ir::Call {
            name: scene_id,
            args: argv,
        })
    }

    fn tr_str(&mut self, t: ast::Str) -> Try<Vec<ir::Expr>> {
        match t {
            ast::Str::Plain(text) => {
                Ok(vec![ir::Expr::Strlit(text.clone())])
            }
        }
    }
}
