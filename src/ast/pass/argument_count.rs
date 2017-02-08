use std::collections::HashMap;

use ast::*;
use ast::visit::*;

use driver::{Try, BuildErr, ErrCtx, BuildErrWithCtx};

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

struct SceneDef {
    args_wanted: usize,
    times_called: usize,
}

struct Pass {
    defs: HashMap<QfdSceneName, SceneDef>,
    context: ErrCtx,
    errors: Vec<BuildErrWithCtx>,
}

impl Pass {
    fn qualify(&self, scene: &SceneName) -> Try<QfdSceneName> {
        Ok(QfdSceneName {
            name: scene.name.clone(),
            in_module: match scene.in_module.as_ref() {
                Some(modpath) => modpath.clone(),
                None => self.context.modpath()?,
            },
        })
    }

    fn push_err(&mut self, err: BuildErr) {
        self.errors.push(BuildErrWithCtx(err, self.context.clone()));
    }

    fn def_scene(&mut self, t: &Scene, modpath: &Modpath) -> Try<()> {
        let &SceneName { ref name, ref in_module } = &t.name;

        let qualified = QfdSceneName {
            name: name.clone(),
            in_module: modpath.clone(),
        };

        self.context = ErrCtx::Local(qualified.clone(), vec![]);

        if in_module.is_some() {
            self.push_err(BuildErr::SceneWasOverqualified(t.name.clone()));
        }

        if self.defs.contains_key(&qualified) {
            self.push_err(BuildErr::SceneWasRedefined(qualified.clone()));
        } else {
            self.defs.insert(qualified, SceneDef {
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
        // Stage 1: Collect scene names
        for &(ref modpath, ref module) in t.modules.iter() {
            for scene in module.scenes.iter() {
                self.def_scene(scene, modpath)?;
            }
        }

        // Stage 2: Check argument counts
        for &(ref modpath, ref module) in t.modules.iter() {
            self.visit_module(module, modpath)?;
        }

        Ok(())
    }

    fn visit_call(&mut self, t: &Call) -> Try<()> {
        let &Call(ref name, ref args) = t;
        let qualified = self.qualify(name)?;

        let err = match self.defs.get_mut(&qualified) {
            Some(def) => {
                def.times_called += 1;
                if args.len() != def.args_wanted {
                    Some(BuildErr::WrongNumberOfArgs {
                        call: t.clone(),
                        wanted: def.args_wanted,
                        got: args.len(),
                    })
                } else {
                    None
                }
            },
            None => { Some(BuildErr::NoSuchScene(qualified.clone())) },
        };

        if let Some(err) = err {
            self.push_err(err);
        }

        Ok(())
    }
}
