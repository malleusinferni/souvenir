//use std::fmt::{Debug, Formatter, Error};

#[derive(Clone, Debug)]
pub struct Module {
    pub globals: Vec<Stmt>,
    pub knots: Vec<Knot>,
}

#[derive(Clone, Debug)]
pub struct Knot {
    pub name: Label,
    pub args: Vec<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Choice {
    pub guard: Expr,
    pub title: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Trap {
    pub pattern: Expr,
    pub guard: Expr,
    pub origin: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Empty,
    Disarm(Label),
    Let(Expr, Expr),
    Listen(Vec<Trap>),
    SendMsg(Expr, Expr),
    LetSpawn(Expr, Label, Vec<Expr>),
    TailCall(Label, Vec<Expr>),
    Trace(Expr),
    Trap(Label, Vec<Trap>),
    Wait(Expr),
    Weave(Label, Vec<Choice>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Label {
    Qualified(Modpath, String),
    Local(String),
    Anonymous,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Modpath(pub Vec<String>);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ActorID(pub u32);

#[derive(Clone, Debug)]
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
    Binop(Box<Expr>, Binop, Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum Binop {
    Roll,
    Add,
    Sub,
    Div,
    Mul,
    Eql,
}

impl<'input> Into<Label> for Option<&'input str> {
    fn into(self) -> Label {
        match self {
            None => Label::Anonymous,
            Some(s) => Label::Local(s.to_owned()),
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
