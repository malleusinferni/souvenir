use ast::*;
use ast::rewrite::*;

use driver::Try;

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

                    output.push(Stmt::Naked {
                        target: target,
                        message: Str::Plain(text),
                    });
                },

                other => output.push(self.rw_stmt(other)?),
            }
        }

        Ok(Block(output))
    }
}
