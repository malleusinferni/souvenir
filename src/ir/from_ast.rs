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
    Disarm,
    Weave,
    WeaveArm(u32),
    Trap,
    TrapArm(u32),
    Listen,
    Trace,
    Let,
    Wait,
    Naked,
    SendMsg,
    Recur,
    Spawn,
}

#[derive(Clone, Debug)]
struct Scope {
    stmt_kind: StKind,
    vars: HashMap<String, Var>,
    errors: Vec<VarErr>,
    next: u32,
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
            next: 0,
            vars: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn child_with(&self, stmt_kind: StKind) -> Self {
        Scope {
            stmt_kind: stmt_kind,

            next: self.next,

            vars: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn next_reg(&mut self) -> ir::Reg {
        let reg = ir::Reg(self.next);
        self.next += 1;
        reg
    }

    fn bind(&mut self, name: String) -> ir::Reg {
        let reg = self.next_reg();

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

        let reg = self.next_reg();
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

    fn assign(&mut self, id: &ast::Ident) -> Try<Option<ir::Reg>> {
        if let &ast::Ident::Var { ref name } = id {
            Ok(Some(self.scope()?.bind(name.clone())))
        } else {
            Err(Error::InvalidAssign(id.clone()))

            // FIXME: Push the error and generate a new name
        }
    }

    fn eval(&mut self, id: &ast::Ident) -> Try<ir::Reg> {
        unimplemented!()
    }

    fn def_label(&mut self, t: &ast::Label) -> Try<()> {
        let value = self.gen_label.next();

        let name = match t {
            &ast::Label::Local { ref name } => name.clone(),
            &ast::Label::Anonymous => format!("[label_{:0x}]", value.0),
        };

        let qualified = self.qualify_label(&name)?;

        if self.labels.contains_key(&qualified) {
            unimplemented!()
        }

        let id = self.gen_label.next();
        self.labels.insert(qualified, id);

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
            &ErrorContext::Global(ref mp) => {
                Err(Error::LabelNotLocal(ast::Label::Local {
                    name: name.to_owned()
                }))
            },

            &ErrorContext::Knot(QualifiedFn(ref modpath, ref func)) => {
                Ok(QualifiedLabel(modpath.clone(), func.clone(), name.to_owned()))
            },
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
                let value = self.tr_expr(value)?;

                if let &ast::Ident::Hole = name {
                    return Ok(Some(ir::Stmt::Discard {
                        value: value,
                    }));
                }

                if let Some(name) = self.assign(name)? {
                    return Ok(Some(ir::Stmt::Let {
                        dest: name,
                        value: value,
                    }));
                }
            },

            other => self.errors.push({
                Error::NotPermittedInGlobalScope(t.clone())
            })
        }

        Ok(None)
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
            let _ = self.assign(arg);
            wanted += 1;
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

        while let Some(stmt) = iter.next() {
            // Reflow text. This is the only scenario where we combine
            // multiple AST statements into one IR statement.
            if let &ast::Stmt::Naked { ref message, ref target } = stmt {
                let mut text = self.tr_naked_str(message)?;

                while let Some(&&ast::Stmt::Naked { ref message, target: None }) = iter.peek() {
                    let next_line = self.tr_naked_str(message)?;
                    text.extend(next_line);

                    iter.next();
                }

                scope.body.push(ir::Stmt::SendMsg {
                    message: unimplemented!(),
                    target: ir::Expr::PidZero,
                });
            } else {
                scope.body.extend(self.tr_stmt(stmt)?);
            }
        }

        self.leave()?;

        Ok(scope)
    }

    fn tr_stmt(&mut self, t: &ast::Stmt) -> Try<Vec<ir::Stmt>> {
        let t = match t {
            &ast::Stmt::Empty => vec![],

            //&ast::Stmt::Disarm { ref target } => vec!{
            //    unimplemented!()
            //},

            &ast::Stmt::Let { ref value, ref name } => vec![{
                let value = self.tr_expr(value)?;

                if let &ast::Ident::Hole = name {
                    ir::Stmt::Discard {
                        value: value,
                    }
                } else if let Some(dest) = self.assign(name)? {
                    ir::Stmt::Let {
                        dest: dest,
                        value: value,
                    }
                } else {
                    return Ok(vec![])
                }
            }],

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

            _ => unimplemented!(),
        };

        Ok(t)
    }

    fn tr_expr(&mut self, t: &ast::Expr) -> Try<ir::Expr> {
        let t = match t {
            &ast::Expr::Id(ref id) => {
                ir::Expr::Id(self.eval(id)?)
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
            _ => unimplemented!()
        };

        Ok(t)
    }

    fn tr_literal(&mut self, t: &ast::Lit) -> Try<ir::Expr> {
        unimplemented!()
    }

    fn tr_fncall(&mut self, t: &ast::FnCall) -> Try<ir::FnCall> {
        unimplemented!()
    }

    fn tr_naked_str(&mut self, t: &ast::Str) -> Try<Vec<ir::Expr>> {
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
