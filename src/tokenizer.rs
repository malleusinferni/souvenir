use std::str::CharIndices;

#[derive(Debug)]
pub struct TokErr {
    pub location: usize,
    pub reason: ErrReason,
}

#[derive(Debug)]
pub enum ErrReason {
    UnrecognizedToken,
    InvalidStringLiteral,
}

fn error<T>(r: ErrReason, l: usize) -> Result<T, TokErr> {
    Err(TokErr { location: l, reason: r })
}

#[derive(Debug, Eq, PartialEq)]
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

    Pipe,
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
                    Some((_, '-')) => {
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
                    Some((_, ' ')) => {
                        Some(self.string_literal(i0))
                    },

                    _ => Some(error(ErrReason::InvalidStringLiteral, i0)),
                },

                '?' => {
                    self.bump();
                    Some(self.screaming_case(i0))
                },

                '\'' => {
                    self.bump();
                    match self.snake_case(i0) {
                        Ok((start, Tok::NmFunc(s), end)) => {
                            Some(Ok((start, Tok::NmLabel(s), end)))
                        },
                        e => Some(e),
                    }
                },

                '#' => {
                    self.bump();
                    match self.snake_case(i0) {
                        Ok((start, Tok::NmFunc(s), end)) => {
                            Some(Ok((start, Tok::LitAtom(s), end)))
                        },
                        e => Some(e),
                    }
                },

                '=' => match self.bump() {
                    Some((i1, '=')) => {
                        self.bump();
                        Some(Ok((i0, Tok::Knot, i1 + 1)))
                    },
                    _ => {
                        Some(Ok((i0, Tok::OpAssign, i0 + 1)))
                    },
                },

                '+' => { self.bump(); Some(Ok((i0, Tok::OpAdd, i0 + 1))) },
                '*' => { self.bump(); Some(Ok((i0, Tok::OpMul, i0 + 1))) },
                '/' => { self.bump(); Some(Ok((i0, Tok::OpDiv, i0 + 1))) },
                '_' => { self.bump(); Some(Ok((i0, Tok::Hole, i0 + 1))) },
                '|' => { self.bump(); Some(Ok((i0, Tok::Pipe, i0 + 1))) },
                ',' => { self.bump(); Some(Ok((i0, Tok::OpComma, i0 + 1))) },

                '(' => { self.bump(); Some(Ok((i0, Tok::LParen, i0 + 1))) },
                ')' => { self.bump(); Some(Ok((i0, Tok::RParen, i0 + 1))) },

                '[' => { self.bump(); Some(Ok((i0, Tok::LSquare, i0 + 1))) },
                ']' => { self.bump(); Some(Ok((i0, Tok::RSquare, i0 + 1))) },

                '{' => { self.bump(); Some(Ok((i0, Tok::LCurly, i0 + 1))) },
                '}' => { self.bump(); Some(Ok((i0, Tok::RCurly, i0 + 1))) },

                c if c.is_alphabetic() => if c.is_lowercase() {
                    Some(self.snake_case(i0))
                } else {
                    unimplemented!()
                },

                c if c.is_digit(10) => {
                    unimplemented!()
                },

                _ => panic!("Can't handle '{}'", c0),
            };
        }
    }

    fn string_literal(&mut self, start: usize) -> TokResult<Tok<'input>> {
        let terminate = |c: char| { c == '\n' };
        let end = self.take_until(terminate).unwrap();
        let contents = &self.text[start .. end];
        Ok((start, Tok::LitStr(contents), end))
    }

    fn snake_case(&mut self, start: usize) -> TokResult<Tok<'input>> {
        let terminate = |c: char| { !(c == '_' || c.is_alphanumeric()) };
        let end = self.take_until(terminate).unwrap();
        let contents = &self.text[start .. end];
        Ok((start, Tok::NmFunc(contents), end))
    }

    fn camel_case(&mut self, start: usize) -> TokResult<Tok<'input>> {
        unimplemented!()
    }

    fn screaming_case(&mut self, start: usize) -> TokResult<Tok<'input>> {
        let terminate = |c: char| { c == '\n' };
        let end = self.take_until(terminate).unwrap();
        let contents = &self.text[start .. end];
        Ok((start, Tok::NmMacro(contents), end))
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

#[test]
fn quick_test() {
    let mut tokenizer = Tokenizer::new("== start\n(ok)#ok\n", 0);

    let expected = &[
        Tok::Knot,
        Tok::NmFunc("start"),
        Tok::EndLn,
        Tok::LParen,
        Tok::NmFunc("ok"),
        Tok::RParen,
        Tok::LitAtom("#ok"),
    ];

    for (wanted, got) in expected.iter().zip(tokenizer) {
        let tok = got.expect("Oh no").1;
        println!("{:?}", &tok);
        assert_eq!(wanted, &tok);
    }
}
