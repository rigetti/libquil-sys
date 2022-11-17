use libquil_sys::{compile_program, get_chip, parse_program, print_program};

const PROGRAM: &str = r#"
DECLARE ro BIT[2]
DECLARE theta REAL
RX(theta) 0
X 0
CNOT 0 1
"#;

fn main() {
    let parsed_program = parse_program(PROGRAM.to_string());
    let chip = get_chip();
    let compiled_program = compile_program(&parsed_program, &chip);
    print_program(&compiled_program)
}
