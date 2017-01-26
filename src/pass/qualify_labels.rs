use ast::rewrite::Rewriter;

use ast::*;

impl Module {
    pub fn qualify_labels(self, modpath: Modpath) -> Result<Self, ()> {
        let mut pass = Pass {
            modpath: modpath,
            next_anon_id: 0,
        };

        pass.rewrite_module(self)
    }
}

struct Pass {
    modpath: Modpath,
    next_anon_id: usize,
}

impl Rewriter<()> for Pass {
    fn rewrite_label(&mut self, t: Label) -> Result<Label, ()> {
        let t = match t {
            Label::Local(name) => {
                Label::Qualified(self.modpath.clone(), name)
            },

            Label::Anonymous => {
                let id = self.next_anon_id;
                self.next_anon_id += 1;
                let name = format!("<anonymous {:0x}>", id);
                Label::Qualified(self.modpath.clone(), name)
            },

            qualified => qualified,
        };

        Ok(t)
    }
}

#[test]
fn example() {
    use parser;
    use tokenizer::Tokenizer;

    let tokens = Tokenizer::new(EXAMPLE_SRC, 0);
    let module = parser::parse_Module(EXAMPLE_SRC, tokens).unwrap();

    let modpath = Modpath(vec!["main".to_owned()]);

    module.qualify_labels(modpath).expect("How did this happen?");
}
