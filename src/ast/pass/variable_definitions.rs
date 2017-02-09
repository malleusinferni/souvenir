use std::collections::HashMap;

use ast::*;
use ast::visit::*;

use driver::{Try, BuildErr, ErrCtx, BuildErrWithCtx};

impl Program {
    pub fn check_variable_definitions(&self) -> Try<()> {
        let mut pass = Pass {
            env: vec![],
            context: ErrCtx::NoContext,
            errors: vec![],
            shadowed: vec![],
        };

        pass.visit_program(&self)?;

        if pass.errors.len() > 0 {
            return Err(pass.errors.into());
        }

        Ok(())
    }
}

struct VarDef {
    uses: usize,
}

struct Scope {
    bindings: HashMap<String, VarDef>,
}

struct Pass {
    env: Vec<Scope>,
    context: ErrCtx,
    errors: Vec<BuildErrWithCtx>,
    shadowed: Vec<String>,
}

impl Visitor for Pass {
    fn enter(&mut self) {
        self.env.push(Scope { bindings: HashMap::new(), });
    }

    fn leave(&mut self) -> Try<()> {
        match self.env.pop() {
            Some(_) => Ok(()),
            None => ice!("Scope underflow"),
        }
    }

    fn visit_id_assign(&mut self, name: &Ident) -> Try<()> {
        let &Ident { ref name } = name;

        let previous = {
            let scope = match self.env.iter_mut().last() {
                Some(s) => s,
                None => ice!("Assignment outside valid scope"),
            };

            scope.bindings.insert(name.to_owned(), VarDef {
                uses: 0,
            })
        };

        match previous {
            Some(ref def) if def.uses < 1 => {
                self.shadowed.push(name.to_owned());
            },

            _ => ()
        }

        Ok(())
    }

    fn visit_id_eval(&mut self, name: &Ident) -> Try<()> {
        let &Ident { ref name } = name;

        for scope in self.env.iter_mut().rev() {
            if let Some(def) = scope.bindings.get_mut(name) {
                def.uses += 1;
            }
        }

        self.errors.push(BuildErr::NoSuchVar(name.to_owned()).with_ctx({
            &self.context
        }));

        Ok(())
    }

    fn error_context(&mut self) -> &mut ErrCtx {
        &mut self.context
    }
}
