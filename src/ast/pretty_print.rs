use std::fmt::*;

use ast;

impl Display for ast::Modpath {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.0.join(":"))
    }
}
