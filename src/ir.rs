#[derive(Copy, Clone, Debug)]
pub enum Op {
    Nop,

    Add(Reg, Reg),
    Sub(Reg, Reg),
    Mul(Reg, Reg),
    Div(Reg, Reg),
    Roll(Reg, Reg),

    /// Initialize register with literal value.
    Init(Val, Reg),

    /// Copy one register to another.
    Mov(Reg, Reg),

    /// Initialize register conditionally.
    Phi(Reg, Reg, Reg),

    /// Set test flags.
    Test(Reg),

    /// Write flag check to register.
    Bool(Cond, Reg),

    /// Invert flag contents.
    Not(Cond),

    /// Undefine temporaries.
    Untemp,

    /// Send buffer contents as a message.
    Msg(Reg),

    /// Print buffer contents to stdout.
    Write,

    /// Wait for a menu selection and jump to the corresponding label.
    Read,

    /// Append buffer contents to the menu.
    AddMenu(Label),

    /// Set condition flags based on menu size.
    CheckMenu,

    /// Append contents of register to the buffer.
    Push(Reg),

    /// Discard buffer contents.
    Nil,

    /// If condition is true, jump to a label.
    Jump(Cond, Label),

    /// Initialize a register with the number of visits to a label.
    Count(Label, Reg),

    /// Spawn a new actor, passing buffer contents as arguments, and save the
    /// resulting actor ID to a register.
    Spawn(Label, Reg),

    /// Undefine all registers, deactivate all traps, and jump to a label,
    /// passing buffer contents as arguments.
    Tail(Label),

    /// Activate the trap at the given label.
    Trap(Label),

    /// Deactiveate the trap at the given label.
    Untrap(Label),

    /// Shut down the current process.
    Graceful,

    /// Halt and catch fire.
    Hcf,
}

#[derive(Copy, Clone, Debug)]
pub enum Reg {
    Mod(u32),
    Var(u32),
    Tmp(u32),
    MyAid,
    Discard,
}

#[derive(Copy, Clone, Debug)]
pub enum Cond {
    /// Always true.
    True,

    /// True if value was zero.
    Zero,

    /// True if value was positive.
    Positive,

    /// True if value was negative.
    Negative,
}

#[derive(Copy, Clone, Debug)]
pub struct Label(u32);

#[derive(Copy, Clone, Debug)]
pub enum Val {
    Undef,
    Int(i32),
    Strid(u32),
    Atom(u32),
    Aid(u32),
    Bid(u32),
}
