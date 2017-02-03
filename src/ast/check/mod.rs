pub mod argument_count;

use ast::*;

#[derive(Clone, Debug)]
pub enum UserErr {
    NoSuchModule(Modpath),
    NoSuchKnot(QfdFnName),
    NameShouldNotBeQualifiedInDef(QfdFnName),
    KnotWasRedefined(QfdFnName),
    WrongNumberOfArgs { wanted: usize, got: usize, },
}

#[derive(Clone, Debug)]
pub struct ICE(pub String);
