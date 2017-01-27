use ast::*;

impl Module {
    pub fn compile(source: &str, path: Modpath) -> Result<Self, String> {
        Module::parse(source).map_err(|err| format!("{:?}", err))?
            .qualify_labels(path).unwrap()
            .resolve_names().map_err(|err| format!("{:?}", err))
    }
}
