#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{
    ffi::{CStr, CString},
    fmt::Display,
    sync::Once,
};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

static START: Once = Once::new();

/// Initializes libquilc using it's core image. No-op after the first call.
fn init_libquilc() {
    let bytes = b"libquilc.core\0".to_vec();
    let mut c_chars: Vec<i8> = bytes.iter().map(|c| *c as i8).collect();
    let ptr = c_chars.as_mut_ptr();
    unsafe {
        dbg!("before");
        init(ptr);
        dbg!("after");
    }
}

/// A quilc chip specification
#[derive(Clone, Debug)]
pub struct Chip(chip_specification);

unsafe impl Send for Chip {}

impl From<CString> for Chip {
    fn from(json: CString) -> Self {
        init_libquilc();

        let mut c_chars: Vec<i8> = json
            .as_bytes_with_nul()
            .to_vec()
            .iter()
            .map(|c| *c as i8)
            .collect();
        let ptr = c_chars.as_mut_ptr();

        unsafe {
            let mut chip: chip_specification = std::ptr::null_mut();
            quilc_parse_chip_spec_isa_json.unwrap()(ptr, &mut chip);
            Chip(chip)
        }
    }
}

/// A parsed Quil program
#[derive(Clone, Debug)]
pub struct Program(quil_program);

unsafe impl Send for Program {}

impl From<CString> for Program {
    fn from(program: CString) -> Self {
        dbg!("hello");
        init_libquilc();
        let mut c_chars: Vec<i8> = program
            .as_bytes_with_nul()
            .to_vec()
            .iter()
            .map(|c| *c as i8)
            .collect();
        let ptr = c_chars.as_mut_ptr();
        let mut parsed_program: quil_program = std::ptr::null_mut();

        dbg!(&c_chars);
        unsafe {
            quilc_parse_quil.unwrap()(ptr, &mut parsed_program);
        }

        Program(parsed_program)
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        init_libquilc();

        unsafe {
            let mut program_string_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
            quilc_program_string.unwrap()(self.0, &mut program_string_ptr);
            let program_string = CStr::from_ptr(program_string_ptr).to_str().unwrap();
            write!(f, "{program_string}")
        }
    }
}

/// Compiles the [`Program`] for the given [`Chip`]
pub fn compile_program(program: &Program, chip: &Chip) -> Program {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
    }

    Program(compiled_program)
}

/// Compiles the [`Program`] for the given [`Chip`] and restricts
/// the resulting [`Program`] to satisfy "protoquil" constraints
pub fn compile_protoquil(program: &Program, chip: &Chip) -> Program {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        quilc_compile_protoquil.unwrap()(program.0, chip.0, &mut compiled_program);
    }

    Program(compiled_program)
}

/// Get a fully-connected 2Q [`Chip`]
pub fn get_chip() -> Chip {
    init_libquilc();
    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
    }

    Chip(chip)
}

/// Prints the given [`Program`] to stdout
pub fn print_program(program: &Program) {
    init_libquilc();
    unsafe {
        quilc_print_program.unwrap()(program.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const sample_quil: &str = "DECLARE ro BIT[2]
DECLARE theta REAL
RX(theta) 0
X 0
CNOT 0 1


MEASURE 0 ro[0]
MEASURE 1 ro[1]
";

    fn new_quil_program() -> Program {
        CString::new(sample_quil).unwrap().into()
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
        let actual: quil_rs::Program = program.to_string().parse().unwrap();
        assert_eq!(actual, expected);
    }
}
