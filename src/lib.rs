pub mod ast;
pub mod parser;
pub mod tokenizer;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use parser;

        let src1 = r#"
        == start
        -- This is a comment and should be ignored
        "#;

        let src2 = r#"
        == start
        weave 'foo
        | > Are you dead?
            > This sucks.
            -> next(A, B)
        | _
            -- Do nothing.
        ;;
        "#;

        parser::parse_Knot(src1).expect("Oh no");
        parser::parse_Module(src2).expect("Oh no");
    }
}
