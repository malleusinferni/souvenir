use std::collections::HashMap;

use ast::*;
use ast::visit::*;

use driver::{Try, BuildErr, ErrCtx};

impl Program {
    pub fn check_names(&self) -> Try<()> {
        let mut pass = Pass {
            defs: HashMap::new(),
            context: ErrCtx::NoContext,
            errors: Vec::new(),
        };

        pass.visit_program(self)?;

        if pass.errors.len() > 0 {
            return Err(pass.errors.into())
        }

        Ok(())
    }
}

struct KnotDef {
    args_wanted: usize,
    times_called: usize,
}

struct Pass {
    defs: HashMap<QfdFnName, KnotDef>,
    context: ErrCtx,
    errors: Vec<(BuildErr, ErrCtx)>,
}

impl Pass {
    fn modpath(&self) -> Try<Modpath> {
        Ok(match &self.context {
            &ErrCtx::NoContext => ice!("Unable to look up module path"),
            &ErrCtx::Prelude(ref path, _) => path.clone(),
            &ErrCtx::KnotDef(ref path, _) => path.clone(),
            &ErrCtx::Local(ref path, _, _) => path.clone(),
        })
    }

    fn qualify(&self, knot_name: &FnName) -> Try<QfdFnName> {
        Ok(QfdFnName {
            name: knot_name.name.clone(),
            in_module: match knot_name.in_module.as_ref() {
                Some(modpath) => modpath.clone(),
                None => self.modpath()?,
            },
        })
    }

    fn push_err(&mut self, err: BuildErr) {
        self.errors.push((err, self.context.clone()));
    }
}

impl Visitor for Pass {
    fn visit_program(&mut self, t: &Program) -> Try<()> {
        // Stage 1: Collect knot names
        for &(ref modpath, ref module) in t.modules.iter() {
            self.context = ErrCtx::Prelude(modpath.clone(), vec![]);
            for knot in module.knots.iter() {
                self.visit_knot(knot)?;
            }
        }

        // Stage 2: Check argument counts
        for &(ref modpath, ref module) in t.modules.iter() {
            self.context = ErrCtx::Prelude(modpath.clone(), vec![]);
            self.visit_module(module)?;
        }

        Ok(())
    }

    fn visit_knot(&mut self, t: &Knot) -> Try<()> {
        let modpath = self.modpath()?;
        self.context = ErrCtx::KnotDef(modpath.clone(), t.name.clone());

        let &FnName { ref name, ref in_module } = &t.name;

        let qualified = QfdFnName {
            name: name.clone(),
            in_module: modpath,
        };

        if in_module.is_some() {
            self.push_err(BuildErr::KnotWasOverqualified);
        }

        if self.defs.contains_key(&qualified) {
            self.push_err(BuildErr::KnotWasRedefined(qualified.clone()));
        } else {
            self.defs.insert(qualified, KnotDef {
                args_wanted: t.args.len(),
                times_called: 0,
            });
        }

        Ok(()) // Don't recurse into knot bodies here
    }

    fn visit_module(&mut self, t: &Module) -> Try<()> {
        self.visit_block(&t.globals)?;

        // Skip visit_knot()!
        for knot in t.knots.iter() {
            self.visit_block(&knot.body)?;
        }

        Ok(())
    }

    fn visit_fncall(&mut self, t: &FnCall) -> Try<()> {
        let &FnCall(ref name, ref args) = t;
        let qualified = self.qualify(name)?;

        let err = match self.defs.get_mut(&qualified) {
            Some(def) => {
                def.times_called += 1;
                if args.len() != def.args_wanted {
                    Some(BuildErr::WrongNumberOfArgs {
                        fncall: t.clone(),
                        wanted: def.args_wanted,
                        got: args.len(),
                    })
                } else {
                    None
                }
            },
            None => { Some(BuildErr::NoSuchKnot(qualified.clone())) },
        };

        if let Some(err) = err {
            self.push_err(err);
        }

        Ok(())
    }
}
