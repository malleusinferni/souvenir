use ast::*;
use ast::visit::*;

use driver::{Try, ErrCtx, BuildErr, BuildErrWithCtx};

impl Program {
    pub fn check_prelude_restrictions(&self) -> Try<()> {
        let mut pass = Pass {
            context: ErrCtx::NoContext,
            errors: Vec::new(),
        };

        pass.visit_program(&self)?;

        if pass.errors.len() > 0 {
            return Err(pass.errors.into());
        }

        Ok(())
    }
}

struct Pass {
    context: ErrCtx,
    errors: Vec<BuildErrWithCtx>,
}

impl Visitor for Pass {
    fn error_context(&mut self) -> &mut ErrCtx {
        &mut self.context
    }

    fn visit_label(&mut self, t: &Label) -> Try<()> {
        if let &ErrCtx::Local(_, _) = &self.context {
            return Ok(());
        }

        self.errors.push(BuildErrWithCtx({
            BuildErr::LabelInPrelude(t.clone())
        }, self.context.clone()));

        Ok(())
    }

    fn visit_ident(&mut self, t: &Ident) -> Try<()> {
        if let &ErrCtx::Local(_, _) = &self.context {
            return Ok(());
        }

        if let &Ident::PidOfSelf = t {
            self.errors.push(BuildErrWithCtx({
                BuildErr::SelfInPrelude
            }, self.context.clone()));
        }

        Ok(())
    }
}
