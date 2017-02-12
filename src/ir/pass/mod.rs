use ir::*;

use driver::Try;

impl Program {
    pub fn optimize(self) -> Try<Self> {
        // TODO: Implement optimizations
        Ok(self)
    }
}
