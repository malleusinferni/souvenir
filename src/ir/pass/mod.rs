use ir::*;

use driver::Try;

impl Program {
    pub fn optimize(self) -> Try<Self> {
        // TODO: Implement optimizations
        Ok(self)
    }
}

// Some basic things to try implementing:
// 1. If block X contains no instructions and ends in an unconditional jump to
//    Y, we can replace all jumps to X with jumps to Y.
// 2. If block X ends in a conditional jump where both branches have the same
//    destination, we can replace it with an unconditional jump.
// 3. If a flag is set and never tested, we can replace the set with a nop.
// 4. We can remove nops.
