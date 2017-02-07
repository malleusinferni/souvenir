pub mod tokens;
pub mod grammar;
pub mod visit;
pub mod check;
pub mod translate;
pub mod pretty_print;

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub modules: Vec<(Modpath, Module)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub globals: Block,
    pub scenes: Vec<Scene>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block(pub Vec<Stmt>);

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub name: SceneName,
    pub args: Vec<Ident>,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeaveArm {
    pub guard: Cond,
    pub message: Expr,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrapArm {
    pub pattern: Pat,
    pub origin: Pat,
    pub guard: Cond,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Empty,

    Disarm {
        target: Label,
    },

    Discard {
        value: Expr,
    },

    Let {
        value: Expr,
        name: Ident,
    },

    Listen {
        name: Label,
        arms: Vec<TrapArm>,
    },

    Naked {
        message: Str,
        target: Option<Ident>,
    },

    Recur {
        target: Call,
    },

    SendMsg {
        message: Expr,
        target: Ident,
    },

    Trace {
        value: Expr,
    },

    Trap {
        name: Label,
        arms: Vec<TrapArm>,
    },

    Wait {
        value: Expr,
    },

    Weave {
        name: Label,
        arms: Vec<WeaveArm>,
    },
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Modpath(pub Vec<String>);

#[derive(Clone, Debug, PartialEq)]
pub struct Call(pub SceneName, pub Vec<Expr>);

#[derive(Clone, Debug, PartialEq)]
pub struct SceneName {
    pub name: String,
    pub in_module: Option<Modpath>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QfdSceneName {
    pub name: String,
    pub in_module: Modpath,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QfdLabel {
    pub name: String,
    pub in_scene: QfdSceneName,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Label {
    Local {
        name: String,
    },

    Anonymous,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ident {
    Var {
        name: String,
    },

    PidOfSelf,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Id(Ident),
    Lit(Lit),
    Str(Str),
    Op(Op, Vec<Expr>),
    List(Vec<Expr>),
    Spawn(Call),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    True,
    False,
    LastResort,
    Compare(BoolOp, Expr, Expr),
    Not(Box<Cond>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Hole,
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
    Roll,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoolOp {
    Eql,
    Gt,
    Lt,
    Gte,
    Lte,
}

use lalrpop_util::ParseError;

use ast::tokens::*;

pub type ParseErr<'a> = ParseError<usize, Tok<'a>, TokErr>;

impl Module {
    pub fn parse(source: &str) -> Result<Self, ParseErr> {
        let tokens = Tokenizer::new(source, 0);

        grammar::parse_Module(source, tokens)
    }
}

impl Default for Pat {
    fn default() -> Self {
        Pat::Hole
    }
}

impl Default for Label {
    fn default() -> Self {
        Label::Anonymous
    }
}
