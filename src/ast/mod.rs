pub mod rewrite;

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub globals: Vec<Stmt>,
    pub knots: Vec<Knot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Knot {
    pub name: Label,
    pub args: Vec<Var>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub guard: Expr,
    pub title: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trap {
    pub pattern: Pat,
    pub guard: Expr,
    pub origin: Pat,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Empty,
    Disarm(Label),
    Let(Assign, Expr),
    Listen(Vec<Trap>),
    SendMsg(Expr, Expr),
    LetSpawn(Assign, FnCall),
    Recur(FnCall),
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

#[derive(Clone, Debug, PartialEq)]
pub struct FnCall(pub Label, pub Vec<Expr>);

#[derive(Clone, Debug, PartialEq)]
pub enum Assign {
    Hole,
    Var(Var),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Var {
    Name(String),
    Register(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Assign(Assign),
    List(Vec<Pat>),
    Literal(Lit),
    Match(Var),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Lit),
    Count(Label),
    Str(String),
    Var(Var),
    Not(Box<Expr>),
    List(Vec<Expr>),
    Binop(Box<Expr>, Binop, Box<Expr>),
    LastResort,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Atom(String),
    Int(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Binop {
    Roll,
    Add,
    Sub,
    Div,
    Mul,
    Eql,
}

impl<'a> From<Option<&'a str>> for Label {
    fn from(input: Option<&'a str>) -> Self {
        match input {
            None => Label::Anonymous,
            Some(s) => Label::Local(s.to_owned()),
        }
    }
}

impl From<bool> for Expr {
    fn from(b: bool) -> Self {
        Expr::Literal({
            if b { Lit::Int(1) } else { Lit::Int(0) }
        })
    }
}

impl<'a> From<&'a str> for Expr {
    fn from(s: &'a str) -> Self {
        Expr::Str(s.to_owned())
    }
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

#[cfg(test)]
pub static EXAMPLE_SRC: &'static str = r#"== knot_name
weave 'foo
| > Option 1
    -> dest1 -- Comment allowed here and ignored
| > Option 2 -- Comment included in string
    -> dest2
| _
    -> dest_default
;;
"#;

#[test]
fn ast_structure() {
    let parsed = Module::parse(EXAMPLE_SRC).unwrap();

    let weave_arms = vec![
        Choice {
            guard: Expr::Literal(Lit::Int(1)),
            title: "Option 1".into(),
            body: vec![
                Stmt::Recur(FnCall(Some("dest1").into(), vec![])),
            ],
        },

        Choice {
            guard: Expr::Literal(Lit::Int(1)),
            title: "Option 2 -- Comment included in string".into(),
            body: vec![
                Stmt::Recur(FnCall(Some("dest2").into(), vec![])),
            ],
        },

        Choice {
            guard: Expr::LastResort,
            title: "".into(),
            body: vec![
                Stmt::Recur(FnCall(Some("dest_default").into(), vec![])),
            ],
        },
    ];

    let expected = Module {
        globals: vec![],
        knots: vec![Knot {
            name: Some("knot_name").into(),
            args: vec![],
            body: vec![
                Stmt::Weave(Some("foo").into(), weave_arms),
            ],
        }],
    };

    if expected == parsed { return; }

    // Pretty print AST so we can compare the output
    panic!("Expected: {:#?}\n\nGot: {:#?}", expected, parsed);
}
