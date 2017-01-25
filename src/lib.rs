extern crate rand;

pub mod ast;
pub mod parser;
pub mod tokenizer;

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

    #[test]
    fn ast_structure() {
        use parser;
        use tokenizer::Tokenizer;

        let src = r#"
        == knot_name
        weave 'foo
        | > Option 1
            -> dest1 -- Comment allowed here and ignored
        | > Option 2 -- Comment included in string
            -> dest2
        | _
            -> dest_default
        ;;
        "#;

        let tokens = Tokenizer::new(src, 0);

        use ast::*;

        let weave_arms = vec![
            Choice {
                guard: Expr::Int(1),
                title: "Option 1".into(),
                body: vec![
                    Stmt::TailCall(Some("dest1").into(), vec![]),
                ],
            },

            Choice {
                guard: Expr::Int(1),
                title: "Option 2 -- Comment included in string".into(),
                body: vec![
                    Stmt::TailCall(Some("dest2").into(), vec![]),
                ],
            },

            Choice {
                guard: Expr::Hole,
                title: "".into(),
                body: vec![
                    Stmt::TailCall(Some("dest_default").into(), vec![]),
                ],
            },
        ];

        let expected = Module {
            globals: vec![],
            knots: vec![Knot {
                name: Some("knot_name").into(),
                args: vec![],
                body: vec![
                    Stmt::Weave(Some("foo").into(), weave_arms),
                    Stmt::Empty,
                ],
            }],
        };

        let parsed = parser::parse_Module(src, tokens).unwrap();

        if expected == parsed { return; }

        panic!("Expected: {:#?}\n\nGot: {:#?}", expected, parsed);
    }
}
