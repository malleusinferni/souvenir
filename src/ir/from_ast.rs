use std::collections::HashMap;

use ast;
use ir;

impl ast::Program {
    pub fn lower(self) -> Result<ir::Program, Vec<Error>> {
        let mut tr = Translator {
            program: ir::Program::default(),
            labels: HashMap::new(),
            gen_label: Counter(0, ir::Label),
            context: None,
            errors: vec![],
            env: vec![],
        };

        tr.translate(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    NameErr(VarErr),
    InvalidInt(String),
    InvalidAssign(ast::Ident),
    LabelNotFound(ast::Label),
    LabelNotLocal(ast::Label),
    LabelRedefined(QualifiedLabel),
    NotPermittedInGlobalScope(ast::Stmt),
    Internal(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorContext {
    Global(ast::Modpath),
    Knot(QualifiedFn),
}

type Try<T> = Result<T, Error>;

type Peek<'i, T> = ::std::iter::Peekable<::std::slice::Iter<'i, T>>;

#[derive(Clone, Debug)]
struct Counter<T>(u32, fn(u32) -> T);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedFn(ast::Modpath, String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedLabel(ast::Modpath, String, String);

struct Translator {
    program: ir::Program,
    labels: HashMap<QualifiedLabel, ir::Label>,
    gen_label: Counter<ir::Label>,
    context: Option<ErrorContext>,
    errors: Vec<Error>,
    env: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VarErr {
    Undefined(String),
    Unused(String),
}

#[derive(Clone, Debug)]
enum StKind {
    Global,
    Knot(String),
    Weave,
    WeaveArm(u32),
    Trap,
    TrapArm(u32),
    Listen,
}

#[derive(Clone, Debug)]
struct Scope {
    stmt_kind: StKind,
    vars: HashMap<String, Var>,
    errors: Vec<VarErr>,
    gen_var: Counter<ir::Reg>,
}

#[derive(Clone, Debug)]
struct Var {
    reg: ir::Reg,
    eval_count: u32,
    defined: bool,
}

impl Scope {
    fn new() -> Self {
        Scope {
            stmt_kind: StKind::Global,
            gen_var: Counter(0, ir::Reg),
            vars: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn child_with(&self, stmt_kind: StKind) -> Self {
        Scope {
            stmt_kind: stmt_kind,

            gen_var: self.gen_var.clone(),

            vars: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn has(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    fn bind(&mut self, name: String) -> ir::Reg {
        let reg = self.gen_var.next();

        let var = Var {
            reg: reg,
            eval_count: 0,
            defined: true,
        };

        if let Some(var) = self.vars.insert(name.clone(), var) {
            if !var.defined {
                self.errors.push(VarErr::Undefined(name));
            } else if var.eval_count < 1 {
                self.errors.push(VarErr::Unused(name))
            }
        }

        reg
    }

    fn eval(&mut self, name: &str) -> ir::Reg {
        if let Some(var) = self.vars.get_mut(name) {
            var.eval_count += 1;
            return var.reg;
        }

        let reg = self.gen_var.next();
        self.vars.insert(name.to_owned(), Var {
            reg: reg,
            eval_count: 1,
            defined: false,
        });

        return reg;
    }
}

impl Translator {
    fn translate(mut self, input: ast::Program) -> Result<ir::Program, Vec<Error>> {
        for (modpath, module) in input.modules {
            self.tr_module(&module, &modpath)?;
        }

        if self.errors.is_empty() {
            Ok(self.program)
        } else {
            Err(self.errors)
        }
    }

    fn enter(&mut self, stmt_kind: StKind) -> Try<()> {
        let new = if let Some(parent) = self.env.iter().last() {
            parent.child_with(stmt_kind)
        } else {
            Scope::new()
        };

        self.env.push(new);

        Ok(())
    }

    fn leave(&mut self) -> Try<()> {
        let scope = self.env.pop()
            .ok_or(Error::Internal(format!("Scope management error")))?;

        for err in scope.errors.into_iter() {
            self.scope()?.errors.push(err);
        }

        Ok(())
    }

    fn scope(&mut self) -> Try<&mut Scope> {
        self.env.iter_mut().last()
            .ok_or(Error::Internal(format!("Scope management error")))
    }

    fn assign(&mut self, name: &str) -> Try<ir::Reg> {
        assert!(name != "Self");
        assert!(name != "_");

        Ok(self.scope()?.bind(name.to_owned()))
    }

    fn eval(&mut self, name: &str) -> Try<ir::Expr> {
        Ok(ir::Expr::Id(self.scope()?.eval(name)))
    }

    fn lookup_var(&self, name: &str) -> bool {
        for scope in self.env.iter().rev() {
            if scope.has(name) { return true; }
        }

        false
    }

    fn def_label(&mut self, t: &ast::Label) -> Try<()> {
        let value = self.gen_label.next();

        let name = match t {
            &ast::Label::Local { ref name } => name.clone(),
            &ast::Label::Anonymous => format!("[label_{:0x}]", value.0),
        };

        let qualified = self.qualify_label(&name)?;

        if self.labels.contains_key(&qualified) {
            self.errors.push(Error::LabelRedefined(qualified));
        } else {
            let id = self.gen_label.next();
            self.labels.insert(qualified, id);
        }

        Ok(())
    }

    fn ref_label(&mut self, t: &ast::Label) -> Try<ir::Label> {
        let name = match t {
            &ast::Label::Local { ref name } => name,
            _ => return Err(Error::Internal({
                format!("Attempted to dereference an anonymous label")
            })),
        };

        let qualified = self.qualify_label(name)?;

        let labels = &mut self.labels;
        let gen = &mut self.gen_label;

        let id = labels.entry(qualified).or_insert_with(|| {
            gen.next()
        }).clone();

        Ok(id)
    }

    fn qualify_label(&mut self, name: &str) -> Try<QualifiedLabel> {
        let ice = Error::Internal(format!("No context for error"));
        match (&self.context).as_ref().ok_or(ice)? {
            &ErrorContext::Global(_) => {
                Err(Error::LabelNotLocal(ast::Label::Local {
                    name: name.to_owned()
                }))
            },

            &ErrorContext::Knot(QualifiedFn(ref modpath, ref func)) => {
                Ok(QualifiedLabel(modpath.clone(), func.clone(), name.to_owned()))
            },
        }
    }

    fn ref_fnid(&mut self, t: &ast::FnName) -> Try<ir::FnId> {
        unimplemented!()
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
            QualifiedFn(p.clone(), name.clone())
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

                // FIXME: Generate the following code:
                //
                //     listen
                //     | #[print finished]
                //     ;;
                //
                // This is surprisingly nontrivial.
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
                let mut t = self.tr_stmt(&ast::Stmt::Trap {
                    name: name.clone(),
                    arms: arms.clone(),
                })?;

                t.push(ir::Stmt::Wait {
                    value: ir::Expr::Infinity,
                });

                t
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
                unimplemented!()
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

            _ => unimplemented!(),
        };

        Ok(t)
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

impl From<Error> for Vec<Error> {
    fn from(e: Error) -> Self {
        vec![e]
    }
}

use std::fmt::{self, Display, Formatter};

impl Display for StKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        unimplemented!()
    }
}

impl<T> Counter<T> {
    fn next(&mut self) -> T {
        let i = self.0;
        self.0 += 1;
        (self.1)(i)
    }
}
