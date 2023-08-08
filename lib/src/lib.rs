#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::{
    ffi::{CStr, CString},
    path::Path,
    str::FromStr,
    sync::Once,
};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error when calling quilc_compile_quil: {0}")]
    CompileQuil(String),
    #[error("error when calling quilc_compile_protoquil: {0}")]
    CompileProtoquil(String),
    #[error("error when calling quilc_parse_quil: {0}")]
    ParseQuil(String),
    #[error("program string contained unexpected NUL character: {0}")]
    UnexpectedNul(#[from] std::ffi::NulError),
    #[error("error when calling quilc_build_nq_linear_chip: {0}")]
    BuildNqLinearChip(String),
    #[error("error when calling quilc_parse_chip_spec_isa_json: {0}")]
    ParseChip(String),
    #[error("error when calling quilc_print_program: {0}")]
    PrintProgram(String),
    #[error("error when calling quilc_program_string: {0}")]
    ProgramString(String),
    #[error("invalid UTF-8 program: {0}")]
    ProgramUtf8(#[from] std::str::Utf8Error),
}

static START: Once = Once::new();

/// Initializes libquilc using it's core image. No-op after the first call.
fn init_libquilc() {
    START.call_once(|| {
        let path = match std::env::var("LIBQUILC_CORE_PATH") {
            Ok(path) => path,
            Err(_) => "libquilc.core".to_string(),
        };
        if !Path::new(&path).exists() {
            // TODO Make this an error rather than a panic
            panic!("Could not find libquilc core file. Do you need to set LIBQUILC_CORE_PATH environment variable?");
        }
        let ptr = CString::new(path).unwrap().into_raw();

        unsafe {
            init(ptr);
        }
    })
}

/// A quilc chip specification
#[derive(Clone, Debug)]
pub struct Chip(chip_specification);

unsafe impl Send for Chip {}

impl TryFrom<CString> for Chip {
    type Error = Error;

    fn try_from(json: CString) -> Result<Self, Self::Error> {
        init_libquilc();

        let ptr = json.into_raw();
        let mut chip: chip_specification = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_chip_spec_isa_json.unwrap()(ptr, &mut chip);
            handle_libquil_error(err).map_err(Error::ParseChip)?;
        }

        Ok(Chip(chip))
    }
}

impl FromStr for Chip {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CString::new(s).map_err(Error::UnexpectedNul)?.try_into()
    }
}

/// A parsed Quil program
#[derive(Clone, Debug)]
pub struct Program(quil_program);

unsafe impl Send for Program {}

impl TryFrom<CString> for Program {
    type Error = Error;

    fn try_from(program: CString) -> Result<Self, Self::Error> {
        init_libquilc();

        let ptr = program.into_raw();
        let mut parsed_program: quil_program = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_quil.unwrap()(ptr, &mut parsed_program);
            handle_libquil_error(err).map_err(Error::ParseQuil)?;
        }

        Ok(Program(parsed_program))
    }
}

impl FromStr for Program {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CString::new(s).map_err(Error::UnexpectedNul)?.try_into()
    }
}

impl Program {
    pub fn to_string(&self) -> Result<String, Error> {
        init_libquilc();

        unsafe {
            let mut program_string_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
            let err = quilc_program_string.unwrap()(self.0, &mut program_string_ptr);
            handle_libquil_error(err).map_err(Error::ProgramString)?;
            let program_string = CStr::from_ptr(program_string_ptr).to_str()?;
            Ok(program_string.to_string())
        }
    }
}

/// Compiles the [`Program`] for the given [`Chip`]
pub fn compile_program(program: &Program, chip: &Chip) -> Result<Program, Error> {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
        handle_libquil_error(err).map_err(Error::CompileQuil)?;
    }

    Ok(Program(compiled_program))
}

/// Compiles the [`Program`] for the given [`Chip`] and restricts
/// the resulting [`Program`] to satisfy "protoquil" constraints
pub fn compile_protoquil(program: &Program, chip: &Chip) -> Result<Program, Error> {
    init_libquilc();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_protoquil.unwrap()(program.0, chip.0, &mut compiled_program);
        handle_libquil_error(err).map_err(Error::CompileProtoquil)?;
    }

    Ok(Program(compiled_program))
}

/// Get a fully-connected 2Q [`Chip`]
// TODO Remove in favor of a better chip builder
pub fn get_chip() -> Result<Chip, Error> {
    init_libquilc();
    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        let err = quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
        handle_libquil_error(err).map_err(Error::BuildNqLinearChip)?;
    }

    Ok(Chip(chip))
}

/// Prints the given [`Program`] to stdout
pub fn print_program(program: &Program) -> Result<(), Error> {
    init_libquilc();

    unsafe {
        let err = quilc_print_program.unwrap()(program.0);
        handle_libquil_error(err).map_err(Error::PrintProgram)?;
    }

    Ok(())
}

fn handle_libquil_error(errno: error_t) -> Result<(), String> {
    if errno == error_t_ERROR_SUCCESS {
        return Ok(());
    }

    let mut error_str_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();

    unsafe {
        let err = quilc_error.unwrap()(&mut error_str_ptr);
        if err != 0 {
            return Err("unknown error occurred".to_string());
        }
        let error_str = CStr::from_ptr(error_str_ptr).to_str().unwrap();
        Err(error_str.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert2::let_assert;

    const sample_quil: &str = "DECLARE ro BIT[2]
DECLARE theta REAL
RX(theta) 0
CNOT 0 1

MEASURE 0 ro[0]
MEASURE 1 ro[1]
";

    fn new_quil_program() -> Program {
        CString::new(sample_quil).unwrap().try_into().unwrap()
    }

    #[test]
    fn test_program_parse_error() {
        let_assert!(Error::ParseQuil(error) = Program::from_str("X 0\n    Y 0").err().unwrap());
        assert!(error.contains("unexpected token of type :INDENT"));
    }

    #[test]
    fn test_program_compilation_error() {
        // Program should parse correctly, but compilation should fail
        // due to the unrecognized instruction
        let program = Program::from_str("GATE").unwrap();
        let_assert!(
            Error::CompileQuil(error) = compile_program(&program, &get_chip().unwrap())
                .err()
                .unwrap()
        );
        assert!(error.contains("Unrecognized instruction"));

        let_assert!(
            Error::CompileProtoquil(error) = compile_protoquil(&program, &get_chip().unwrap())
                .err()
                .unwrap()
        );
        assert!(error.contains("Unrecognized instruction"));
    }

    #[test]
    fn test_compile_protoquil() {
        let program = new_quil_program();
        let chip = get_chip().unwrap();
        compile_protoquil(&program, &chip).unwrap();
    }

    #[test]
    fn test_program_string() {
        let expected: quil_rs::Program = sample_quil.parse().unwrap();
        let program = new_quil_program();
        let actual: quil_rs::Program = program.to_string().unwrap().parse().unwrap();
        assert_eq!(actual, expected);
    }
}
