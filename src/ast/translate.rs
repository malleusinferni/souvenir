use std::collections::HashMap;

use ast;
use ir;

use driver::{Try, ErrCtx, BuildErr};

type Peek<'i, T> = ::std::iter::Peekable<::std::slice::Iter<'i, T>>;

struct Translator {
    program: ir::Program,
    //labels: HashMap<ast::QfdLabel, ir::Label>,
    //gen_label: Counter<ir::Label>,
    context: ErrCtx,
    errors: Vec<(BuildErr, ErrCtx)>,
}

impl Translator {
    fn translate(mut self, input: ast::Program) -> Try<ir::Program> {
        for (modpath, module) in input.modules {
            self.tr_module(&module, &modpath)?;
        }

        if self.errors.is_empty() {
            Ok(self.program)
        } else {
            Err(self.errors.into())
        }
    }

    fn tr_module(&mut self, t: &ast::Module, p: &ast::Modpath) -> Try<()> {
        self.context = Some(ErrorContext::Global(p.clone()));

        self.enter(StKind::Global)?;

        let mut prelude = ir::Scope { body: vec![] };
        for stmt in t.globals.0.iter() {
            if let Some(stmt) = self.tr_global(stmt)? {
                prelude.body.push(stmt);
            }
        }

        for knot in t.knots.iter() {
            self.tr_knot(&knot, p)?;
        }

        self.leave()?;

        Ok(())
    }

    fn tr_global(&mut self, t: &ast::Stmt) -> Try<Option<ir::Stmt>> {
        match t {
            &ast::Stmt::Empty => (),

            &ast::Stmt::Let { ref value, ref name } => {
                match self.tr_let(name, value) {
                    Ok(stmt) => return Ok(Some(stmt)),

                    Err(err@Error::InvalidAssign(_)) => {
                        self.errors.push(err);
                    },

                    Err(other) => return Err(other),
                }
            },

            _ => self.errors.push({
                Error::NotPermittedInGlobalScope(t.clone())
            })
        }

        Ok(None)
    }

    fn tr_let(&mut self, n: &ast::Ident, v: &ast::Expr) -> Try<ir::Stmt> {
        let value = self.tr_expr(v)?;

        let t = match n {
            &ast::Ident::Hole => ir::Stmt::Discard {
                value: value,
            },

            &ast::Ident::PidOfSelf => {
                return Err(Error::InvalidAssign(n.clone()))
            },

            &ast::Ident::Var { ref name } => ir::Stmt::Let {
                dest: self.assign(name)?,
                value: value,
            },
        };

        Ok(t)
    }

    fn tr_knot(&mut self, t: &ast::Knot, p: &ast::Modpath) -> Try<()> {
        let name = &t.name.name;
        assert!(t.name.in_module.is_none());

        self.context = Some(ErrorContext::Knot({
            QfdFn(p.clone(), name.clone())
        }));

        self.enter(StKind::Knot(name.clone()))?;

        let mut wanted = 0;
        for arg in &t.args {
            wanted += 1;

            let _reg: ir::Reg = match arg {
                &ast::Ident::Var { ref name } => self.assign(name)?,

                &ast::Ident::Hole => continue, // ?????

                &ast::Ident::PidOfSelf => {
                    self.errors.push(Error::InvalidAssign(arg.clone()));
                    continue;
                },
            };
        }

        let body = self.tr_block(&t.body, StKind::Knot(name.clone()))?;

        self.leave()?;

        self.program.knots.push(ir::KnotDef {
            args_wanted: wanted,
            body: body,
        });

        Ok(())
    }

    fn tr_block(&mut self, t: &ast::Block, k: StKind) -> Try<ir::Scope> {
        self.enter(k)?;

        let &ast::Block(ref block) = t;

        let mut scope = ir::Scope { body: vec![] };
        let mut iter = block.iter().peekable();

        while iter.peek().is_some() {
            // Text reflow is the only operation that combines multiple AST
            // statements into a single IR statement. All other desugaring
            // operations produce larger output than input.
            if let Some(&&ast::Stmt::Naked { .. }) = iter.peek() {
                scope.body.push(self.reflow(&mut iter)?);
                scope.body.push(self.gen_wait_after_write()?);
            } else {
                for stmt in self.tr_stmt(iter.next().unwrap())? {
                    scope.body.push(stmt);
                }
            }
        }

        self.leave()?;

        Ok(scope)
    }

    fn reflow(&mut self, iter: &mut Peek<ast::Stmt>) -> Try<ir::Stmt> {
        let (target, topic, mut text) = match iter.next() {
            Some(&ast::Stmt::Naked { ref target, ref message }) => {
                let target = match target.as_ref() {
                    Some(id) => {
                        let id = ast::Expr::Id(id.clone());
                        self.tr_expr(&id)?
                    },
                    None => ir::Expr::PidZero,
                };

                // FIXME: Support other topics eventually
                let topic = ir::Atom::PrintLine;

                let message = self.tr_str(message)?;

                (target, topic, message)
            },

            other => return Err(Error::Internal({
                format!("Unexpected {:?} when reflowing text", other)
            })),
        };

        while iter.peek().is_some() {
            match iter.peek().expect("Unreachable") {
                &&ast::Stmt::Naked { target: None, ref message } => {
                    text.extend(self.tr_str(message)?);
                },

                _ => break,
            }

            let _ = iter.next();
        }

        Ok(ir::Stmt::SendMsg {
            target: target,
            message: ir::Expr::List({
                vec![
                    ir::Expr::Atom(topic),
                    ir::Expr::Strcat(text),
                ]
            }),
        })
    }

    fn tr_stmt(&mut self, t: &ast::Stmt) -> Try<Vec<ir::Stmt>> {
        let t = match t {
            &ast::Stmt::Empty => vec![],

            &ast::Stmt::Disarm { ref target } => {
                let _ = self.ref_label(target)?;
                vec![unimplemented!()]
            },

            &ast::Stmt::Let { ref value, ref name } => vec![
                self.tr_let(name, value)?,
            ],

            &ast::Stmt::Listen { ref name, ref arms } => {
                unimplemented!()
            },

            &ast::Stmt::Naked { .. } => {
                return Err(Error::Internal({
                    format!("Forgot to reflow a print statement: {:?}", t)
                }))
            },

            &ast::Stmt::Recur { ref target } => {
                vec![ir::Stmt::Recur {
                    target: self.tr_fncall(target)?,
                }]
            },

            &ast::Stmt::SendMsg { ref message, ref target } => {
                let target = ast::Expr::Id(target.clone());
                vec![ir::Stmt::SendMsg {
                    target: self.tr_expr(&target)?,
                    message: self.tr_expr(message)?,
                }]
            },

            &ast::Stmt::Trace { ref value } => {
                vec![ir::Stmt::Trace {
                    value: self.tr_expr(value)?,
                }]
            },

            &ast::Stmt::Wait { ref value } => {
                vec![ir::Stmt::Wait {
                    value: self.tr_expr(value)?,
                }]
            },

            &ast::Stmt::Weave { ref name, ref arms } => {
                unimplemented!()
            },

            &ast::Stmt::Trap { ref name, ref arms } => {
                vec![self.tr_trap(name, arms)?]
            },
        };

        Ok(t)
    }

    fn tr_trap(&mut self, n: &ast::Label, t: &Vec<ast::TrapArm>) -> Try<ir::Stmt> {
        unimplemented!()
    }

    fn tr_expr(&mut self, t: &ast::Expr) -> Try<ir::Expr> {
        let t = match t {
            &ast::Expr::Id(ref id) => match id {
                &ast::Ident::Hole => {
                    unimplemented!()
                },

                &ast::Ident::PidOfSelf => {
                    ir::Expr::PidOfSelf
                },

                &ast::Ident::Var { ref name } => {
                    self.eval(name)?
                },
            },

            &ast::Expr::Lit(ref lit) => {
                self.tr_literal(lit)?
            },

            &ast::Expr::Str(_) => {
                unimplemented!()
            },

            &ast::Expr::Op(_, _) => {
                unimplemented!()
            },

            &ast::Expr::List(ref items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.tr_expr(item)?);
                }
                ir::Expr::List(out)
            },

            &ast::Expr::Spawn(ref call) => {
                ir::Expr::Spawn(self.tr_fncall(call)?)
            },
        };

        Ok(t)
    }

    fn tr_pat(&mut self, t: &ast::Pat) -> Try<ir::Pat> {
        let t = match t {
            &ast::Pat::Id(ref id) => match id {
                &ast::Ident::Hole => ir::Pat::Hole,

                &ast::Ident::PidOfSelf => ir::Pat::EqualTo({
                    ir::Expr::PidOfSelf
                }),

                &ast::Ident::Var { ref name } => {
                    if self.lookup_var(name) {
                        ir::Pat::EqualTo(self.eval(name)?)
                    } else {
                        ir::Pat::Assign(self.assign(name)?)
                    }
                },
            },

            &ast::Pat::Lit(ref lit) => {
                ir::Pat::EqualTo(self.tr_literal(lit)?)
            },

            &ast::Pat::List(ref items) => {
                let mut pats = Vec::with_capacity(items.len());
                for item in items.iter() {
                    pats.push(self.tr_pat(item)?);
                }
                ir::Pat::List(pats)
            },
        };

        Ok(t)
    }

    fn tr_literal(&mut self, t: &ast::Lit) -> Try<ir::Expr> {
        let t = match t {
            &ast::Lit::Atom(ref name) => {
                ir::Expr::Atom(ir::Atom::User(name.clone()))
            },

            &ast::Lit::Int(n) => {
                ir::Expr::Int(n)
            },

            &ast::Lit::InvalidInt(ref digits) => {
                self.errors.push(Error::InvalidInt(digits.clone()));
                ir::Expr::Int(i32::default())
            },
        };

        Ok(t)
    }

    fn tr_fncall(&mut self, t: &ast::FnCall) -> Try<ir::FnCall> {
        let &ast::FnCall(ref name, ref args) = t;

        let fnid = self.ref_fnid(name)?;

        let mut argv = Vec::with_capacity(args.len());
        for arg in args {
            argv.push(self.tr_expr(arg)?);
        }

        Ok(ir::FnCall(fnid, argv))
    }

    fn tr_str(&mut self, t: &ast::Str) -> Try<Vec<ir::Expr>> {
        match t {
            &ast::Str::Plain(ref text) => {
                Ok(vec![ir::Expr::Strlit(text.clone())])
            }
        }
    }
}
