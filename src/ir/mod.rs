pub mod rewrite;
pub mod pretty_print;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    pub preludes: Vec<Scope>,
    pub scenes: Vec<SceneDef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneDef {
    pub prelude_id: usize,
    pub args_wanted: u32,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: Pat,
    pub guard: Cond,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrapArm {
    pub pattern: Pat,
    pub sender: Pat,
    pub guard: Cond,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeaveArm {
    pub guard: Cond,
    pub message: Expr,
    pub body: Scope,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub body: Vec<Stmt>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SugarKind {
    Listen,
    Match,
    Naked,
    Trap,
    Weave,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SugarStmt {
    Listen {
        label: Label,
        arms: Vec<TrapArm>,
    },

    Match {
        value: Expr,
        arms: Vec<MatchArm>,
        failure: Scope,
    },

    Naked {
        target: Expr,
        topic: Option<Expr>,
        text: Vec<Expr>, // TODO
    },

    Trap {
        label: Label,
        arms: Vec<TrapArm>,
    },

    Weave {
        label: Label,
        arms: Vec<WeaveArm>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Desugared {
        from: SugarKind,
        stmts: Vec<Stmt>,
    },

    Sugar {
        stmt: SugarStmt,
    },

    Arm {
        name: Label,
        body: Scope,
    },

    Disarm {
        name: Label,
    },

    Discard {
        value: Expr,
    },

    If {
        test: Cond,
        success: Scope,
        failure: Scope,
    },

    Let {
        value: Expr,
        dest: Var,
    },

    Recur {
        target: Call,
    },

    Return {
        result: bool,
    },

    SendMsg {
        message: Expr,
        target: Expr,
    },

    Trace {
        value: Expr,
    },

    Wait {
        value: Expr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Atom(Atom),
    Var(Var),
    Int(i32),
    List(Vec<Expr>),
    Nth(Box<Expr>, u32),
    Spawn(Call),
    Strcat(Vec<Expr>),
    Strlit(String),
    FetchArgument,
    PidOfSelf,
    PidZero,
    Infinity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    True,
    False,
    LastResort,
    HasLength(Expr, u32),
    Equals(Expr, Expr),
    AllOf(Vec<Cond>),
    AnyOf(Vec<Cond>),
    Not(Box<Cond>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Hole,
    Assign(Var),
    EqualTo(Expr),
    List(Vec<Pat>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Atom {
    MenuItem,
    MenuEnd,
    LastResort,
    PrintLine,
    PrintFinished,
    User(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub name: SceneId,
    pub args: Vec<Expr>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Label(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub enum LabelName {
    User(String),
    Generated(u32),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Var {
    Id(String),
    Gen(u32),
    Reg(u32),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SceneId(pub u32);

impl From<Vec<Stmt>> for Scope {
    fn from(body: Vec<Stmt>) -> Self {
        Scope { body: body, }
    }
}
