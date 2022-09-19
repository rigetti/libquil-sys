#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

struct Chip(chip_specification);

struct Program(quil_program);

fn parse_program(program: String) -> Program {
    let mut c_chars: Vec<i8> = program
        .as_bytes()
        .to_vec()
        .iter()
        .map(|c| *c as i8)
        .collect();
    let ptr = c_chars.as_mut_ptr();
    let mut parsed_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_parse_quil.unwrap()(ptr, &mut parsed_program);
    }

    Program(parsed_program)
}

fn compile_program(program: &Program, chip: &Chip) -> Program {
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
    }

    Program(compiled_program)
}

fn get_chip() -> Chip {
    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
    }

    Chip(chip)
}

fn print_program(program: &Program) {
    unsafe {
        quilc_print_program.unwrap()(program.0);
    }
}

fn init_libquilc() {
    let bytes = b"libquilc.core\0".to_vec();
    let mut c_chars: Vec<i8> = bytes.iter().map(|c| *c as i8).collect();
    let ptr = c_chars.as_mut_ptr();
    unsafe {
        init(ptr);
    }
}

fn main() {
    init_libquilc();

    let program = parse_program("H 0".to_string());
    print_program(&program);
    let chip = get_chip();
    let program = compile_program(&program, &chip);
    print_program(&program);
}
