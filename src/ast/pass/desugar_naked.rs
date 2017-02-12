use ast::*;
use ast::rewrite::*;

use driver::Try;

impl Program {
    pub fn desugar_naked(mut self) -> Try<Self> {
        for &mut (_, ref mut module) in self.modules.iter_mut() {
            module.reflow_text()?;
        }

        Ok(self)
    }
}

impl Module {
    pub fn reflow_text(&mut self) -> Try<()> {
        // I only did it this way for the sake of writing tests like below
        let mut scenes = Vec::with_capacity(self.scenes.len());
        for scene in self.scenes.drain(..) {
            let mut pass = Pass;
            let scene = pass.rw_scene(scene)?;
            scenes.push(scene);
        }

        self.scenes = scenes;

        Ok(())
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

#[test]
fn reflow() {
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
}
