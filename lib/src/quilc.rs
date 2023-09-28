use crate::{
    bindings::{
        chip_specification, quil_program, quilc_build_nq_linear_chip, quilc_compile_protoquil,
        quilc_compile_quil, quilc_conjugate_pauli_by_clifford, quilc_generate_rb_sequence,
        quilc_get_version_info, quilc_parse_chip_spec_isa_json, quilc_parse_quil,
        quilc_print_program, quilc_program_string, quilc_version_info, quilc_version_info_githash,
        quilc_version_info_version,
    },
    init_libquil,
};
use std::{
    ffi::{CStr, CString},
    fmt::Display,
    str::FromStr,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error when calling quilc_compile_quil: {0}")]
    CompileQuil(String),
    #[error("error when calling quilc_compile_protoquil: {0}")]
    CompileProtoquil(String),
    #[error("error when calling quilc_conjugate_pauli_by_clifford: {0}")]
    ConjugatePauliByClifford(String),
    #[error("error when calling generate_rb_sequence: {0}")]
    GenerateRbSequence(String),
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
/// A quilc chip specification
#[derive(Clone, Debug)]
pub struct Chip(chip_specification);

// The Chip memory held by libquil is never mutated and
// is thus `Send`.
unsafe impl Send for Chip {}

impl TryFrom<CString> for Chip {
    type Error = Error;

    fn try_from(json: CString) -> Result<Self, Self::Error> {
        crate::init_libquil();

        let ptr = json.into_raw();
        let mut chip: chip_specification = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_chip_spec_isa_json.unwrap()(ptr, &mut chip);
            crate::handle_libquil_error(err).map_err(Error::ParseChip)?;
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
pub struct Program(pub(crate) quil_program);

// The Program memory held by libquil is never mutated and
// is thus `Send`.
unsafe impl Send for Program {}

impl TryFrom<CString> for Program {
    type Error = Error;

    fn try_from(program: CString) -> Result<Self, Self::Error> {
        init_libquil();

        let ptr = program.into_raw();
        let mut parsed_program: quil_program = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_quil.unwrap()(ptr, &mut parsed_program);
            crate::handle_libquil_error(err).map_err(Error::ParseQuil)?;
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
        init_libquil();

        unsafe {
            let mut program_string_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
            let err = quilc_program_string.unwrap()(self.0, &mut program_string_ptr);
            crate::handle_libquil_error(err).map_err(Error::ProgramString)?;
            let program_string = CStr::from_ptr(program_string_ptr).to_str()?;
            Ok(program_string.to_string())
        }
    }
}

/// Compiles the [`Program`] for the given [`Chip`]
pub fn compile_program(program: &Program, chip: &Chip) -> Result<Program, Error> {
    init_libquil();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
        crate::handle_libquil_error(err).map_err(Error::CompileQuil)?;
    }

    Ok(Program(compiled_program))
}

/// Compiles the [`Program`] for the given [`Chip`] and restricts
/// the resulting [`Program`] to satisfy "protoquil" constraints
pub fn compile_protoquil(program: &Program, chip: &Chip) -> Result<Program, Error> {
    init_libquil();
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_protoquil.unwrap()(program.0, chip.0, &mut compiled_program);
        crate::handle_libquil_error(err).map_err(Error::CompileProtoquil)?;
    }

    Ok(Program(compiled_program))
}

/// Get a fully-connected 2Q [`Chip`]
pub fn get_chip() -> Result<Chip, Error> {
    init_libquil();
    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        let err = quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
        crate::handle_libquil_error(err).map_err(Error::BuildNqLinearChip)?;
    }

    Ok(Chip(chip))
}

/// Prints the given [`Program`] to stdout
pub fn print_program(program: &Program) -> Result<(), Error> {
    init_libquil();

    unsafe {
        let err = quilc_print_program.unwrap()(program.0);
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct ConjugatePauliByCliffordResult {
    pub phase: i32,
    pub pauli: String,
}

pub fn conjugate_pauli_by_clifford(
    mut pauli_indices: Vec<u32>,
    mut pauli_terms: Vec<String>,
    clifford: &Program,
) -> Result<ConjugatePauliByCliffordResult, Error> {
    init_libquil();

    unsafe {
        let mut phase = 0;
        let phase_ptr = std::ptr::addr_of_mut!(phase);
        let pauli_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = quilc_conjugate_pauli_by_clifford.unwrap()(
            pauli_indices.as_mut_ptr() as *mut _,
            pauli_indices.len() as i32,
            pauli_terms.as_mut_ptr() as *mut _,
            pauli_terms.len() as i32,
            clifford.0,
            phase_ptr as *mut _,
            std::ptr::addr_of!(pauli_ptr) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::ConjugatePauliByClifford)?;
        Ok(ConjugatePauliByCliffordResult {
            phase,
            pauli: CStr::from_ptr(pauli_ptr).to_str()?.to_string(),
        })
    }
}

pub fn generate_rb_sequence(
    depth: i32,
    qubits: i32,
    gateset: Vec<&Program>,
    seed: i32,
    interleaver: Option<&Program>,
) -> Result<Vec<Vec<i32>>, Error> {
    init_libquil();

    let mut gateset = gateset.iter().map(|p| p.0).collect::<Vec<_>>();
    let mut results_ptr: *mut std::ffi::c_int = std::ptr::null_mut();
    let results_ptr_ptr = std::ptr::addr_of_mut!(results_ptr);
    // If there is an interleaver program, it is placed between each of the sequences indices,
    // thus extending the sequence by (depth - 1).
    let result_lens_len = if interleaver.is_none() {
        depth
    } else {
        2 * depth - 1
    };
    let mut result_lens = vec![0_i32; result_lens_len as usize];

    let interleaver = if let Some(interleaver) = interleaver {
        std::ptr::addr_of!(interleaver.0)
    } else {
        std::ptr::null_mut()
    };

    unsafe {
        let err = quilc_generate_rb_sequence.unwrap()(
            depth,
            qubits,
            gateset.as_mut_ptr() as *mut _,
            gateset.len() as i32,
            seed,
            interleaver as *mut _,
            results_ptr_ptr as *mut _,
            result_lens.as_mut_ptr() as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::GenerateRbSequence)?;
    }

    let n_sequences: i32 = result_lens.iter().sum();
    let results = unsafe { std::slice::from_raw_parts(results_ptr, n_sequences as usize) }.to_vec();
    let mut results_iter = results.into_iter();
    let collected_results = result_lens
        .into_iter()
        .map(|l| results_iter.by_ref().take(l as usize).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    Ok(collected_results)
}

#[derive(Debug)]
pub struct VersionInfo {
    pub version: String,
    pub githash: String,
}

impl Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "quilc {} ({})", self.version, self.githash)
    }
}

pub fn get_version_info() -> Result<VersionInfo, Error> {
    init_libquil();

    unsafe {
        let mut version_info: quilc_version_info = std::ptr::null_mut();
        let err = quilc_get_version_info.unwrap()(&mut version_info);
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        let mut version_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = quilc_version_info_version.unwrap()(version_info, &mut version_ptr);
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        let mut githash_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = quilc_version_info_githash.unwrap()(version_info, &mut githash_ptr);
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        Ok(VersionInfo {
            version: CStr::from_ptr(version_ptr).to_str()?.to_string(),
            githash: CStr::from_ptr(githash_ptr).to_str()?.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    // use crate::bindings::quilc_get_version_info;

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

    #[test]
    fn test_get_version_info() {
        get_version_info().unwrap();
    }

    #[test]
    fn test_conjugate_pauli_by_clifford() {
        let pauli_indices = vec![0];
        let x = "X".to_string();
        let clifford = "H 0".parse().unwrap();

        let expected = ConjugatePauliByCliffordResult {
            phase: 0,
            pauli: "Z".to_string(),
        };
        let result = conjugate_pauli_by_clifford(pauli_indices, vec![x], &clifford).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_rb_sequence_with_interleaver() {
        let phase = "PHASE(pi/2) 0".parse().unwrap();
        let h = "H 0".parse().unwrap();
        let y = "Y 0".parse().unwrap();
        let interleaver = Some(&y);

        let expected = vec![vec![0, 1], vec![2], vec![0, 0, 0, 1], vec![2], vec![0, 1]];
        let results = generate_rb_sequence(3, 1, vec![&phase, &h, &y], 42, interleaver).unwrap();
        assert_eq!(results, expected);
    }

    #[test]
    fn test_generate_rb_sequence_without_interleaver() {
        let phase = "PHASE(pi/2) 0".parse().unwrap();
        let h = "H 0".parse().unwrap();
        let y = "Y 0".parse().unwrap();
        let interleaver = None;

        let expected = vec![vec![2, 0, 1], vec![0, 0, 0, 1], vec![0, 1]];
        let results = generate_rb_sequence(3, 1, vec![&phase, &h, &y], 42, interleaver).unwrap();
        assert_eq!(results, expected);
    }
}
