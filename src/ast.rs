//use std::fmt::{Debug, Formatter, Error};

#[derive(Debug)]
pub struct Module {
    pub globals: Vec<Stmt>,
    pub knots: Vec<Knot>,
}

#[derive(Debug)]
pub struct Knot {
    pub name: Label,
    pub args: Vec<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Choice {
    pub guard: Expr,
    pub title: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct Trap {
    pub pattern: Expr,
    pub guard: Expr,
    pub origin: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Stmt {
    Empty,
    Disarm(Label),
    Listen(Vec<Trap>),
    SendMsg(Expr, Expr),
    TailCall(Label, Vec<Expr>),
    Trace(Expr),
    Trap(Label, Vec<Trap>),
    Wait(Expr),
    Weave(Label, Vec<Choice>),
}

#[derive(Debug)]
pub enum Label {
    Explicit(String),
    Anonymous,
}

#[derive(Debug)]
pub struct ActorID(pub u32);

#[derive(Debug)]
pub enum Expr {
    Hole,
    Actor(ActorID),
    Count(Label),
    Atom(String),
    Var(String),
    Str(String),
    Int(i32),
    Not(Box<Expr>),
    List(Vec<Expr>),
    Spawn(Label, Vec<Expr>),
    Binop(Box<Expr>, Binop, Box<Expr>),
}

#[derive(Debug)]
pub enum Binop {
    Roll,
    Add,
    Sub,
    Div,
    Mul,
}

impl<'input> Into<Label> for Option<&'input str> {
    fn into(self) -> Label {
        match self {
            None => Label::Anonymous,
            Some(s) => Label::Explicit(s.to_owned()),
        }
    }
}

impl Expr {
    pub fn lit_true() -> Self {
        Expr::Int(1)
    }

    pub fn lit_false() -> Self {
        Expr::Int(0)
    }

    pub fn lit_string(input: &str) -> Self {
        Expr::Str(input.to_owned())
    }
}
