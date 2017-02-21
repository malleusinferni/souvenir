pub mod argument_count;
pub mod prelude_restrictions;
pub mod variable_definitions;

pub mod qualify_modpaths;

pub mod desugar_listen;
pub mod desugar_match;
pub mod desugar_weave;
pub mod desugar_trap;
pub mod desugar_naked;

use ast::*;

use driver::Try;

#[derive(Clone, Debug, PartialEq)]
pub struct DesugaredProgram {
    pub preludes: Vec<(Modpath, Block)>,
    pub scenes: Vec<Scene>,
    pub lambdas: Vec<TrapLambda>,
}

impl Program {
    pub fn desugar(self) -> Try<DesugaredProgram> {
        let mut preludes = vec![];
        let mut scenes = vec![];

        for (modpath, module) in self.modules {
            let prelude_body = module.globals.qualify(&modpath)?;

            for scene in module.scenes {
                scenes.push(scene.qualify(&modpath)?);
            }

            preludes.push((modpath, prelude_body));
        }

        let dst = DesugaredProgram {
            preludes: preludes,
            scenes: scenes,
            lambdas: vec![],
        };

        dst.desugar_naked()?
            .desugar_listen()?
            .desugar_trap()?
            .desugar_weave()?
            .desugar_match()
    }
}

impl SceneName {
    pub fn qualified(&self) -> Try<QfdSceneName> {
        match self.in_module.as_ref() {
            Some(modpath) => Ok(QfdSceneName {
                name: self.name.clone(),
                in_module: modpath.clone(),
            }),

            None => ice!("Expected scene name {} to be qualified", self.name),
        }
    }
}

impl Label {
    pub fn qualified(&self) -> Try<QfdLabel> {
        match self {
            &Label::Qualified(ref q) => Ok(q.clone()),
            _ => ice!("Expected label name {} to be qualified", self),
        }
    }
}
