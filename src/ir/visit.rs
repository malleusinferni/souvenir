use ir::*;

use driver::Try;

pub trait Visitor {
    fn visit_program(&mut self, prog: &Program) -> Try<()> {
        for block in prog.blocks.iter() {
            self.visit_block(block)?;
        }

        Ok(())
    }

    fn visit_block(&mut self, block: &Block) -> Try<()> {
        for op in block.ops.iter() {
            self.visit_op(op)?;
        }

        self.visit_exit(&block.exit)
    }

    fn visit_op(&mut self, op: &Op) -> Try<()> {
        match op {
            &Op::Arm(ref trap_ref) => {
                self.visit_label(&trap_ref.label)?;
                self.visit_var_read(&trap_ref.env)?;
            },

            &Op::Disarm(ref label) => {
                self.visit_label(label)?;
            },

            &Op::Export(ref env, ref var) => {
                self.visit_var_read(var)?;
            },

            &Op::Let(ref name, ref value) => {
                self.visit_rval(value)?;
                self.visit_var_write(name)?;
            },

            &Op::Listen(ref trap_ref) => {
                self.visit_label(&trap_ref.label)?;
                self.visit_var_read(&trap_ref.env)?;
            },

            &Op::Say(ref var) => {
                self.visit_var_read(var)?;
            },

            &Op::Store(ref var, ref ptr) => {
                self.visit_var_read(&ptr.start_addr)?;
                self.visit_var_write(var)?;
            },

            &Op::SendMsg(ref lhs, ref rhs) => {
                self.visit_var_read(rhs)?;
                self.visit_var_read(lhs)?;
            },

            &Op::Set(ref flag, ref tval) => {
                self.visit_tval(tval)?;
                self.visit_flag(flag)?;
            },

            &Op::Trace(ref var) => {
                self.visit_var_read(var)?;
            },

            &Op::Wait(ref var) => {
                self.visit_var_read(var)?;
            },

            &Op::Write(ref var) => {
                self.visit_var_read(var)?;
            },
        }

        Ok(())
    }

    fn visit_exit(&mut self, exit: &Exit) -> Try<()> {
        match exit {
            &Exit::EndProcess => (),

            &Exit::Goto(ref label) => self.visit_label(label)?,

            &Exit::IfThenElse(ref flag, ref succ, ref fail) => {
                self.visit_flag(flag)?;
                self.visit_label(succ)?;
                self.visit_label(fail)?;
            },

            &Exit::Recur(ref call) => {
                self.visit_label(&call.label)?;
                self.visit_var_read(&call.argv)?;
            },

            &Exit::Return(_) => (),
        }

        Ok(())
    }

    fn visit_rval(&mut self, rval: &Rvalue) -> Try<()> {
        match rval {
            &Rvalue::Var(ref var) => {
                self.visit_var_read(var)?;
            },

            &Rvalue::Arg(_) => (),
            &Rvalue::Int(_) => (),

            &Rvalue::Add(ref lhs, ref rhs) => {
                self.visit_var_read(lhs)?;
                self.visit_var_read(rhs)?;
            },

            &Rvalue::Sub(ref lhs, ref rhs) => {
                self.visit_var_read(lhs)?;
                self.visit_var_read(rhs)?;
            },

            &Rvalue::Div(ref lhs, ref rhs) => {
                self.visit_var_read(lhs)?;
                self.visit_var_read(rhs)?;
            },

            &Rvalue::Mul(ref lhs, ref rhs) => {
                self.visit_var_read(lhs)?;
                self.visit_var_read(rhs)?;
            },

            &Rvalue::Roll(ref lhs, ref rhs) => {
                self.visit_var_read(lhs)?;
                self.visit_var_read(rhs)?;
            },

            &Rvalue::Load(ref ptr) => {
                self.visit_var_read(&ptr.start_addr)?;
            },

            &Rvalue::LoadEnv(_) => (),

            &Rvalue::FromBool(ref flag) => {
                self.visit_flag(flag)?;
            },

            &Rvalue::Spawn(ref call) => {
                self.visit_label(&call.label)?;
                self.visit_var_read(&call.argv)?;
            },

            &Rvalue::Splice(ref vars) => {
                for var in vars.iter() {
                    self.visit_var_read(var)?;
                }
            },

            &Rvalue::Alloc(_) => (),

            &Rvalue::Const(_) => (),

            &Rvalue::MenuChoice(ref var) => {
                self.visit_var_read(var)?;
            },

            &Rvalue::PidOfSelf => (),
        }

        Ok(())
    }

    fn visit_tval(&mut self, tval: &Tvalue) -> Try<()> {
        Ok(())
    }

    fn visit_flag(&mut self, flag: &Flag) -> Try<()> {
        Ok(())
    }

    fn visit_label(&mut self, label: &Label) -> Try<()> {
        Ok(())
    }

    fn visit_var_read(&mut self, var: &Var) -> Try<()> {
        Ok(())
    }

    fn visit_var_write(&mut self, var: &Var) -> Try<()> {
        Ok(())
    }
}
