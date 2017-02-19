use ast::*;
use ast::pass::*;
use ast::rewrite::*;

use driver::Try;

impl DesugaredProgram {
    pub fn desugar_naked(self) -> Try<Self> {
        Pass.rw_desugared(self)
    }
}

struct Pass;

impl Rewriter for Pass {
    fn rw_block(&mut self, t: Block) -> Try<Block> {
        let Block(mut stack) = t;
        let mut output = Vec::with_capacity(stack.len());

        stack.reverse();

        while let Some(stmt) = stack.pop() {
            match stmt {
                Stmt::Naked { target, message } => {
                    let mut text = match message {
                        Str::Plain(text) => text,
                    };

                    while let Some(stmt) = stack.pop() {
                        match stmt {
                            Stmt::Naked {
                                target: Expr::PidZero,
                                message: Str::Plain(next_line),
                            } => {
                                text.push(' ');
                                text.push_str(&next_line);
                            },

                            other => {
                                stack.push(other);
                                break;
                            },
                        }
                    }

                    let () = match target {
                        Expr::PidZero => (),
                        _other => ice!("SayVia: Not yet supported"),
                    };

                    output.push(Stmt::Say {
                        message: Expr::Str(Str::Plain(text)),
                    });
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}

#[test]
fn reflow() {
    // FIXME: This test won't compile anymore.

    /*
    let before = r"
    == start
    > This
    > should
    > all
    > be
    > on
    > one
    > line.
    ";

    let after = r"
    == start
    > This should all be on one line.
    ";

    let mut before_parsed = Module::parse(before).unwrap();
    before_parsed.reflow_text().unwrap();

    let after_parsed = Module::parse(after).unwrap();

    assert_eq!(before_parsed, after_parsed);
    */
}
