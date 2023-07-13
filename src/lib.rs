#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{ffi::CStr, sync::Once};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[derive(Debug)]
pub struct Chip(chip_specification);

#[derive(Debug)]
pub struct Program(quil_program);

static START: Once = Once::new();

/// Initializes libquilc using it's core image. No-op after the first call.
fn init_libquilc() {
    START.call_once(|| {
        let bytes = b"libquilc.core\0".to_vec();
        let mut c_chars: Vec<i8> = bytes.iter().map(|c| *c as i8).collect();
        let ptr = c_chars.as_mut_ptr();
        unsafe {
            init(ptr);
        }
    })
}

/// Parses a String into a Program object for use with other libquil calls.
pub fn parse_program(program: String) -> Program {
    init_libquilc();
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

/// Compiles the program, optimized for the given Chip.
pub fn compile_program(program: &Program, chip: &Chip) -> Program {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
    }

    Program(compiled_program)
}

pub fn compile_protoquil(program: &Program, chip: &Chip) -> Program {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_compile_protoquil.unwrap()(program.0, chip.0, &mut compiled_program);
    }

    Program(compiled_program)
}

/// Get an arbritrary Chip.
pub fn get_chip() -> Chip {
    init_libquilc();
    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
    }

    Chip(chip)
}

/// Prints the given Program to stdout
pub fn print_program(program: &Program) {
    init_libquilc();
    unsafe {
        quilc_print_program.unwrap()(program.0);
    }
}

pub fn program_string(program: &Program) -> String {
    init_libquilc();

    unsafe {
        let mut program_string_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        quilc_program_string.unwrap()(program.0, &mut program_string_ptr);
        let program_string = CStr::from_ptr(program_string_ptr)
            .to_string_lossy()
            .to_string();
        program_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const sample_quil: &str = r#"
DECLARE ro BIT[2]
DECLARE theta REAL
RX(theta) 0
X 0
CNOT 0 1
MEASURE 0 ro[0]
MEASURE 1 ro[1]
"#;

    fn new_quil_program() -> Program {
        parse_program(sample_quil.to_string())
    }

    #[test]
    fn test_compile_protoquil() {
        let program = new_quil_program();
        let chip = get_chip();
        compile_protoquil(&program, &chip);
    }

    #[test]
    fn test_program_string() {
        let expected: quil_rs::Program = sample_quil.parse().unwrap();
        let program = new_quil_program();
        let actual: quil_rs::Program = program_string(&program).parse().unwrap();
        assert_eq!(actual, expected);
    }
}
