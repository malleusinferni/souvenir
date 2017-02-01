pub mod visit;

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub globals: Block,
    pub knots: Vec<Knot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block(pub Vec<Stmt>);

#[derive(Clone, Debug, PartialEq)]
pub struct Knot {
    pub name: Ident,
    pub args: Vec<Ident>,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeaveArm {
    pub guard: Expr,
    pub message: Expr,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrapArm {
    pub pattern: Pat,
    pub origin: Pat,
    pub guard: Expr,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Empty,

    Disarm {
        target: Ident,
    },

    Let {
        value: Expr,
        name: Ident,
    },

    Listen {
        name: Ident,
        arms: Vec<TrapArm>,
    },

    Naked {
        message: Str,
        target: Option<Ident>,
    },

    Recur {
        target: FnCall,
    },

    SendMsg {
        message: Expr,
        target: Ident,
    },

    Trace {
        value: Expr,
    },

    Trap {
        name: Ident,
        arms: Vec<TrapArm>,
    },

    Wait {
        value: Expr,
    },

    Weave {
        name: Ident,
        arms: Vec<WeaveArm>,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Modpath(pub Vec<String>);

#[derive(Clone, Debug, PartialEq)]
pub struct FnCall(pub Ident, pub Vec<Expr>);

#[derive(Clone, Debug, PartialEq)]
pub enum Ident {
    Func {
        name: String,
        in_module: Option<Modpath>,
    },

    Label {
        name: String,
    },

    AnonymousLabel,

    PidOfSelf,

    Var {
        name: String,
    },

    Hole,

    Invalid(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Id(Ident),
    Lit(Lit),
    Str(Str),
    Op(Op, Vec<Expr>),
    List(Vec<Expr>),
    Spawn(FnCall),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Id(Ident),
    Lit(Lit),
    List(Vec<Pat>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Atom(String),
    Int(i32),
    InvalidInt(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Str {
    Plain(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Div,
    Mul,
    Eql,
    Gt,
    Lt,
    Gte,
    Lte,
    Not,
    Roll,
}

use lalrpop_util::ParseError;

use tokenizer::*;

pub type ParseErr<'a> = ParseError<usize, Tok<'a>, TokErr>;

impl Module {
    pub fn parse(source: &str) -> Result<Self, ParseErr> {
        let tokens = Tokenizer::new(source, 0);

        ::parser::parse_Module(source, tokens)
    }
}

pub trait IdentOption {
    fn or_hole(self) -> Ident;
    fn or_label(self) -> Ident;
}

impl IdentOption for Option<Ident> {
    fn or_hole(self) -> Ident {
        self.unwrap_or(Ident::Hole)
    }

    fn or_label(self) -> Ident {
        self.unwrap_or(Ident::AnonymousLabel)
    }
}

pub trait ExprOption {
    fn or_true(self) -> Expr;
    fn or_false(self) -> Expr;
}

impl ExprOption for Option<Expr> {
    fn or_true(self) -> Expr {
        self.unwrap_or(Expr::Lit(Lit::Int(1)))
    }

    fn or_false(self) -> Expr {
        self.unwrap_or(Expr::Lit(Lit::Int(0)))
    }
}
