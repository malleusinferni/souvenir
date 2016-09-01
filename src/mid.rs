use std::collections::HashMap;

use rand::Rng;

use ast::*;

use eval::rem::Supervisor;

pub struct TreeWalker {
    _phony: (),
}

impl TreeWalker {
    pub fn new(modules: HashMap<Modpath, Module>) -> Self {
        let _ = modules;
        TreeWalker { _phony: () }
    }
}
