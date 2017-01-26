extern crate rand;
extern crate lalrpop_util;

pub mod ast;
pub mod parser;
pub mod tokenizer;

pub mod pass;

#[test]
fn it_works() {
    use parser;
    use tokenizer::Tokenizer;

    let src1 = r#"== start
    -- This is a comment and should be ignored
    "#;

    let tokens1 = Tokenizer::new(src1, 0);
    parser::parse_Module(src1, tokens1).expect("Oh no");

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

    let tokens2 = Tokenizer::new(src2, 0);
    parser::parse_Module(src2, tokens2).expect("Oh no");
}
