use std::str::CharIndices;

pub struct TokErr {
    pub location: usize,
    pub reason: ErrReason,
}

pub enum ErrReason {
    UnrecognizedToken,
    InvalidStringLiteral,
}

fn error<T>(r: ErrReason, l: usize) -> Result<T, TokErr> {
    Err(TokErr { location: l, reason: r })
}

pub enum Tok<'input> {
    EndLn,
    EndBlk,

    KwDisarm,
    KwFrom,
    KwGiven,
    KwIf,
    KwListen,
    KwThen,
    KwTrace,
    KwTrap,
    KwWait,
    KwWeave,
    KwWhen,

    NmFunc(&'input str),
    NmLabel(&'input str),
    NmMacro(&'input str),
    NmVar(&'input str),

    LitAtom(&'input str),
    LitInt(&'input str),
    LitRoll(&'input str),
    LitStr(&'input str),

    OpAssign,
    OpComma,
    OpDot,
    OpSend,

    OpMul,
    OpDiv,
    OpAdd,
    OpSub,

    Bar,
    Hole,
    Knot,

    LParen,
    RParen,
    LSquare,
    RSquare,
    LCurly,
    RCurly,
    LAngle,
    RAngle,
}

pub struct Tokenizer<'input> {
    text: &'input str,
    chars: CharIndices<'input>,
    lookahead: Option<(usize, char)>,
    shift: usize,
}

macro_rules! eof {
    ($x:expr) => {
        match $x { Some(v) => v, None => { return None; } }
    }
}

pub type Spanned<T> = (usize, T, usize);
pub type TokResult<T> = Result<Spanned<T>, TokErr>;

const KEYWORDS: &'static [(&'static str, Tok<'static>)] = &[
    ("disarm", Tok::KwDisarm),
    ("from", Tok::KwFrom),
    ("given", Tok::KwGiven),
    ("if", Tok::KwIf),
    ("listen", Tok::KwListen),
    ("then", Tok::KwThen),
    ("trace", Tok::KwTrace),
    ("trap", Tok::KwTrap),
    ("wait", Tok::KwWait),
    ("weave", Tok::KwWeave),
    ("when", Tok::KwWhen),
];

impl<'input> Tokenizer<'input> {
    pub fn new(text: &'input str, shift: usize) -> Self {
        let mut t = Tokenizer {
            text: text,
            chars: text.char_indices(),
            lookahead: None,
            shift: shift,
        };

        t.bump();

        t
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        self.lookahead = self.chars.next();
        self.lookahead
    }

    fn take_until<F>(&mut self, mut terminate: F) -> Option<usize>
        where F: FnMut(char) -> bool
    {
        while let Some((i, c)) = self.lookahead {
            if terminate(c) { return Some(i); }
            self.bump();
        }

        None
    }

    fn next_unshifted(&mut self) -> Option<TokResult<Tok<'input>>> {
        loop {
            let (i0, c0) = match self.lookahead {
                Some(ic) => ic,
                None => return None,
            };

            return match c0 {
                ' ' | '\t' => {
                    self.bump();
                    continue
                },

                '\n' => {
                    self.bump();
                    Some(Ok((i0, Tok::EndLn, i0 + 1)))
                },

                ';' => match self.bump() {
                    Some((i1, ';')) => {
                        self.bump();
                        Some(Ok((i0, Tok::EndBlk, i1 + 1)))
                    },

                    _ => Some(Ok((i0, Tok::EndLn, i0 + 1))),
                },

                '-' => match self.bump() {
                    Some((i1, '-')) => {
                        let i_n = self.take_until(|c| c == '\n').unwrap();
                        Some(Ok((i0, Tok::EndLn, i_n)))
                    },

                    _ => Some(Ok((i0, Tok::OpSub, i0 + 1))),
                },

                '<' => match self.bump() {
                    Some((i1, '-')) => {
                        self.bump();
                        Some(Ok((i0, Tok::OpSend, i1 + 1)))
                    },

                    _ => Some(Ok((i0, Tok::LAngle, i0 + 1))),
                },

                '>' => match self.bump() {
                    Some((i1, ' ')) => {
                        Some(self.string_literal(i0))
                    },

                    _ => Some(error(ErrReason::InvalidStringLiteral, i0)),
                },

                c if c.is_alphanumeric() => {
                    unimplemented!()
                },

                _ => unimplemented!(),
            };
        }
    }

    fn string_literal(&mut self, start: usize) -> TokResult<Tok<'input>> {
        unimplemented!()
    }
}

impl<'input> Iterator for Tokenizer<'input> {
    type Item = TokResult<Tok<'input>>;

    fn next(&mut self) -> Option<Self::Item> {
        let h = self.shift;

        match self.next_unshifted() {
            None => None,

            Some(Ok((l, t, r))) => Some(Ok((l+h, t, r+h))),

            Some(Err(TokErr { location, reason })) =>
                Some(Err(TokErr { location: location+h, reason: reason })),
        }
    }
}
