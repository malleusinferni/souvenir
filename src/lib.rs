extern crate rand;
extern crate lalrpop_util;

#[macro_use]
pub mod driver;

pub mod ast;
pub mod parser;
pub mod tokenizer;

pub mod ir;

pub mod vm;

#[test]
fn parse_examples() {
    use ast::Module;

    let src1 = r#"== start
    -- This is a comment and should be ignored
    "#;

    Module::parse(src1).expect("Example 1 failed");

    let src2 = r#"== start
    weave 'foo
    | > Option 1
        > Prints something.
    | > Option 2
        -- Prints nothing.
    | _
        > Should not even appear!
    ;;
    "#;

    Module::parse(src2).expect("Example 2 failed");

    let src3 = r#"
    let Four = 2 + 2
    let B = spawn util:timeout(5 * 5)
    == start
    B <- #time_to_die
    "#;

    Module::parse(src3).expect("Example 3 failed");
}
