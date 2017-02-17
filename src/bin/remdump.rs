extern crate souvenir;
extern crate string_interner;

fn main() {
    use souvenir::vm;

    let code = vec![
        vm::Instr::Add(vm::Reg(0), vm::Reg(1)),
        vm::Instr::JumpIf(vm::Flag(99), vm::Label(101)),
        vm::Instr::Bye,
    ];

    let jump_table = vec![
        vm::InstrAddr(0),
    ];

    let program = vm::Program {
        code: code.into(),
        jump_table: jump_table.into(),
        atom_table: string_interner::StringInterner::new(),
        str_table: string_interner::StringInterner::new(),
        env_table: vec![].into(),
    };

    println!("{}", program);
}
