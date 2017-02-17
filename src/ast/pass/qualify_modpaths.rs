use ast::*;
use ast::rewrite::*;

use driver::Try;

enum Ctx {
    Prelude(Modpath),
    Scene(QfdSceneName),
}

struct Pass {
    context: Ctx,
    label_gen: Counter<String>,
}

fn deanonymize_label(id: u32) -> String {
    format!("anonymous_label%{:x}", id)
}

impl Block {
    pub fn qualify(self, modpath: &Modpath) -> Try<Self> {
        let mut pass = Pass {
            context: Ctx::Prelude(modpath.clone()),
            label_gen: Counter(0, deanonymize_label),
        };

        pass.rw_block(self)
    }
}

impl Scene {
    pub fn qualify(self, modpath: &Modpath) -> Try<Self> {
        let qfd_name = QfdSceneName {
            name: self.name.name.clone(),
            in_module: modpath.clone(),
        };

        let mut pass = Pass {
            context: Ctx::Scene(qfd_name),
            label_gen: Counter(0, deanonymize_label),
        };

        Ok(Scene {
            name: SceneName {
                name: self.name.name,
                in_module: Some(modpath.clone()),
            },
            args: self.args,
            body: pass.rw_block(self.body)?,
        })
    }
}

impl Pass {
    fn scene_name(&self) -> Try<QfdSceneName> {
        match &self.context {
            &Ctx::Prelude(_) => ice!("Label encountered in prelude"),
            &Ctx::Scene(ref scene_name) => Ok(scene_name.clone()),
        }
    }

    fn module_name(&self) -> Try<Modpath> {
        match &self.context {
            &Ctx::Prelude(ref path) => Ok(path.clone()),
            &Ctx::Scene(ref scene_name) => Ok(scene_name.in_module.clone()),
        }
    }
}

impl Rewriter for Pass {
    fn rw_label(&mut self, t: Label) -> Try<Label> {
        Ok(match t {
            Label::Local { name } => Label::Qualified(QfdLabel {
                name: name,
                in_scene: self.scene_name()?,
            }),

            Label::Anonymous => Label::Qualified(QfdLabel {
                name: self.label_gen.next(),
                in_scene: self.scene_name()?,
            }),

            Label::Qualified(q) => Label::Qualified(q),
        })
    }

    fn rw_scene_name(&mut self, mut t: SceneName) -> Try<SceneName> {
        if t.in_module.is_none() {
            t.in_module = Some(self.module_name()?);
        }

        Ok(t)
    }
}
