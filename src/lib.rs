extern crate rand;

pub mod ast;
pub mod parser;
pub mod tokenizer;
pub mod eval;
pub mod ir;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use parser;
        use tokenizer::Tokenizer;

        let src1 = r#"== start
        -- This is a comment and should be ignored
        "#;

        let tokens1 = Tokenizer::new(src1, 0);
        parser::parse_Knot(src1, tokens1).expect("Oh no");

        let src2 = r#"== start
        weave 'foo
        | > Are you dead?
            > This sucks.
            -> next(A, B)
        | _
            -- Do nothing.
        ;;
        "#;

        let tokens2 = Tokenizer::new(src2, 0);
        parser::parse_Module(src2, tokens2).expect("Oh no");
    }
}
