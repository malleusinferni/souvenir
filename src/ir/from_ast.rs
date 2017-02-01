use std::collections::HashMap;

use ast;
use ir;

impl ast::Program {
    pub fn lower(self) -> Result<ir::Program, Vec<Error>> {
        let mut tr = Translator {
            program: ir::Program::default(),
            labels: HashMap::new(),
            next_label: 0,
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
    LabelRedefined(QualifiedLabel),
    Internal(String),
}

type Try<T> = Result<T, Error>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct QualifiedFn(ast::Modpath, String);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedLabel(ast::Modpath, String, String);

struct Translator {
    program: ir::Program,
    labels: HashMap<QualifiedLabel, ir::Label>,
    next_label: u32,
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
    Global(ast::Modpath),
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
    modpath: ast::Modpath,
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
    fn new(modpath: &ast::Modpath) -> Self {
        Scope {
            modpath: modpath.clone(),
            stmt_kind: StKind::Global(modpath.clone()),
            next: 0,
            vars: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn child_with(&self, stmt_kind: StKind) -> Self {
        Scope {
            stmt_kind: stmt_kind,

            modpath: self.modpath.clone(),
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

    fn enter(&mut self, stmt_kind: StKind) {
        let new = if let Some(parent) = self.env.iter().last() {
            parent.child_with(stmt_kind)
        } else {
            let mp = match &stmt_kind {
                &StKind::Global(ref mp) => mp,
                _ => unreachable!(),
            };

            Scope::new(mp)
        };

        self.env.push(new);
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

    fn tr_module(&mut self, t: &ast::Module, p: &ast::Modpath) -> Try<()> {
        self.enter(StKind::Global(p.clone()));

        for knot in &t.knots {
            self.tr_knot(&knot)?;
        }

        self.leave()?;

        Ok(())
    }

    fn tr_knot(&mut self, t: &ast::Knot) -> Try<()> {
        let name = &t.name.name;
        assert!(t.name.in_module.is_none());

        self.enter(StKind::Knot(name.clone()));

        let mut wanted = 0;
        for arg in &t.args {
            let _ = self.assign(arg);
            wanted += 1;
        }

        let body = self.tr_block(&t.body)?;

        self.leave()?;

        self.program.knots.push(ir::KnotDef {
            args_wanted: wanted,
            body: body,
        });

        Ok(())
    }

    fn tr_block(&mut self, t: &ast::Block) -> Try<ir::Scope> {
        unimplemented!()
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
