use std::collections::HashMap;

use ast::*;
use ast::visit::*;

use driver::{BuildErr, ICE};

impl Program {
    pub fn check_names(&self) -> Result<Result<(), Vec<BuildErr>>, ICE> {
        let mut pass = Pass {
            defs: HashMap::new(),
            current_modpath: None,
            errors: Vec::new(),
        };

        pass.visit_program(self)?;

        Ok(if pass.errors.is_empty() {
            Ok(())
        } else {
            Err(pass.errors)
        })
    }
}

struct KnotDef {
    args_wanted: usize,
    times_called: usize,
}

struct Pass {
    defs: HashMap<QfdFnName, KnotDef>,
    current_modpath: Option<Modpath>,
    errors: Vec<BuildErr>,
}

impl Pass {
    fn modpath(&self) -> Result<Modpath, ICE> {
        self.current_modpath.as_ref().cloned().ok_or({
            ICE(format!("Module path was not set up"))
        })
    }

    fn qualify(&self, knot_name: &FnName) -> Result<QfdFnName, ICE> {
        Ok(QfdFnName {
            name: knot_name.name.clone(),
            in_module: match knot_name.in_module.as_ref() {
                Some(modpath) => modpath.clone(),
                None => self.modpath()?,
            },
        })
    }
}

impl Visitor for Pass {
    fn visit_program(&mut self, t: &Program) -> Result<(), ICE> {
        // Stage 1: Collect knot names
        for &(ref modpath, ref module) in t.modules.iter() {
            self.current_modpath = Some(modpath.clone());
            for knot in module.knots.iter() {
                self.visit_knot(knot)?;
            }
        }

        // Stage 2: Check argument counts
        for &(ref modpath, ref module) in t.modules.iter() {
            self.current_modpath = Some(modpath.clone());
            self.visit_module(module)?;
        }

        Ok(())
    }

    fn visit_knot(&mut self, t: &Knot) -> Result<(), ICE> {
        let modpath = self.modpath()?;

        let &FnName { ref name, ref in_module } = &t.name;

        let qualified = QfdFnName {
            name: name.clone(),
            in_module: modpath,
        };

        if in_module.is_some() {
            self.errors.push(BuildErr::NameShouldNotBeQualifiedInDef({
                qualified.clone()
            }));
        }

        if self.defs.contains_key(&qualified) {
            self.errors.push(BuildErr::KnotWasRedefined(qualified.clone()));
        } else {
            self.defs.insert(qualified, KnotDef {
                args_wanted: t.args.len(),
                times_called: 0,
            });
        }

        Ok(()) // Don't recurse into knot bodies here
    }

    fn visit_module(&mut self, t: &Module) -> Result<(), ICE> {
        self.visit_block(&t.globals)?;

        // Skip visit_knot()!
        for knot in t.knots.iter() {
            self.visit_block(&knot.body)?;
        }

        Ok(())
    }

    fn visit_fncall(&mut self, t: &FnCall) -> Result<(), ICE> {
        let &FnCall(ref name, ref args) = t;
        let qualified = self.qualify(name)?;

        let err = match self.defs.get_mut(&qualified) {
            Some(def) => {
                def.times_called += 1;
                if args.len() != def.args_wanted {
                    Some(BuildErr::WrongNumberOfArgs {
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
            self.errors.push(err);
        }

        Ok(())
    }
}
