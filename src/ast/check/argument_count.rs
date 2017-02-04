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
    fn qualify(&self, knot_name: &FnName) -> Try<QfdFnName> {
        Ok(QfdFnName {
            name: knot_name.name.clone(),
            in_module: match knot_name.in_module.as_ref() {
                Some(modpath) => modpath.clone(),
                None => self.context.modpath()?,
            },
        })
    }

    fn push_err(&mut self, err: BuildErr) {
        self.errors.push((err, self.context.clone()));
    }

    fn def_knot(&mut self, t: &Knot, modpath: &Modpath) -> Try<()> {
        let &FnName { ref name, ref in_module } = &t.name;

        let qualified = QfdFnName {
            name: name.clone(),
            in_module: modpath.clone(),
        };

        self.context = ErrCtx::Local(qualified.clone(), vec![]);

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

        Ok(())
    }
}

impl Visitor for Pass {
    fn error_context(&mut self) -> &mut ErrCtx {
        &mut self.context
    }

    fn visit_program(&mut self, t: &Program) -> Try<()> {
        // Stage 1: Collect knot names
        for &(ref modpath, ref module) in t.modules.iter() {
            for knot in module.knots.iter() {
                self.def_knot(knot, modpath)?;
            }
        }

        // Stage 2: Check argument counts
        for &(ref modpath, ref module) in t.modules.iter() {
            self.visit_module(module, modpath)?;
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
