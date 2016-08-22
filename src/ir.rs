pub use ast::ActorID;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Op {
    Nop,

    /// r3 = bop(r1, r2)
    Binop(Binop, Reg, Reg, Reg),

    /// Initialize register with int value.
    LitInt(i32, Reg),

    /// Initialize register with string value.
    LitStr(u32, Reg),

    /// Copy one register to another.
    Mov(Reg, Reg),

    /// Initialize register conditionally.
    Phi(Reg, Reg, Reg),

    /// Compare two values.
    Cmp(Reg, Reg),

    /// "Compare" a single value.
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
    Bye,

    /// Halt and catch fire.
    Hcf,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Reg {
    /// Module-level global variable.
    Mod(u32),

    /// Argument or local variable.
    Var(u32),

    /// Temporary variable.
    Tmp(u32),

    /// The `Self` variable.
    MyAid,

    /// The `_` variable ("hole").
    Discard,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Binop {
    Add,
    Mul,
    Sub,
    Div,
    Roll,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Label(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Val {
    Int(i32),
    Strid(u32),
    Atom(u32),
    Aid(ActorID),
    List(Vec<Val>),
    Strseq(Vec<Val>),
}
