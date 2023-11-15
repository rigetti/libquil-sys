use crate::{
    bindings::{
        self, chip_specification, quil_program, quilc_build_nq_linear_chip,
        quilc_compilation_metadata, quilc_compile_protoquil, quilc_compile_quil,
        quilc_conjugate_pauli_by_clifford, quilc_generate_rb_sequence, quilc_get_version_info,
        quilc_parse_chip_spec_isa_json, quilc_parse_quil, quilc_print_program,
        quilc_program_memory_type, quilc_program_string, quilc_version_info,
        quilc_version_info_githash, quilc_version_info_version,
    },
    get_string_from_pointer_and_free, init_libquil,
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
    #[error("error when getting compilation metadata: {0}")]
    CompilationMetadata(String),
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
    #[error("failed to initialize libquil: {0}")]
    FailedToInitializeLibquil(#[from] crate::Error),
    #[error("failed to get memory type in program: {0}")]
    ProgramMemoryType(String),
    #[error("unknown memory type: {0}")]
    UnknownMemoryType(u32),
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
        crate::init_libquil()?;

        let ptr = json.into_raw();
        let mut chip: chip_specification = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_chip_spec_isa_json.unwrap()(ptr, &mut chip);
            crate::handle_libquil_error(err).map_err(Error::ParseChip)?;
            let _ = CString::from_raw(ptr);
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

impl Drop for Chip {
    fn drop(&mut self) {
        unsafe {
            bindings::lisp_release_handle.unwrap()(self.0 as *mut _);
        }
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
        init_libquil()?;

        let ptr = program.into_raw();
        let mut parsed_program: quil_program = std::ptr::null_mut();

        unsafe {
            let err = quilc_parse_quil.unwrap()(ptr, &mut parsed_program);
            crate::handle_libquil_error(err).map_err(Error::ParseQuil)?;
            let _ = CString::from_raw(ptr);
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

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { bindings::lisp_release_handle.unwrap()(self.0 as *mut _) }
    }
}

impl Program {
    pub fn to_string(&self) -> Result<String, Error> {
        init_libquil()?;

        unsafe {
            let mut program_string_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
            let err = quilc_program_string.unwrap()(
                self.0,
                std::ptr::addr_of_mut!(program_string_ptr) as *mut _,
            );
            crate::handle_libquil_error(err).map_err(Error::ProgramString)?;
            let program_string = get_string_from_pointer_and_free(program_string_ptr)?;
            Ok(program_string)
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MemoryType {
    Bit,
    Octet,
    Integer,
    Real,
}

impl TryFrom<i32> for MemoryType {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value as u32 {
            bindings::program_memory_type_t_LIBQUIL_TYPE_BIT => Ok(Self::Bit),
            bindings::program_memory_type_t_LIBQUIL_TYPE_OCTET => Ok(Self::Octet),
            bindings::program_memory_type_t_LIBQUIL_TYPE_INTEGER => Ok(Self::Integer),
            bindings::program_memory_type_t_LIBQUIL_TYPE_REAL => Ok(Self::Real),
            x => Err(Error::UnknownMemoryType(x)),
        }
    }
}

pub fn program_memory_type(program: &Program, region: &str) -> Result<MemoryType, Error> {
    init_libquil()?;

    unsafe {
        let region_cstr = CString::new(region)?;
        let mut region_type = 0;
        let err = quilc_program_memory_type.unwrap()(
            program.0,
            region_cstr.into_raw(),
            std::ptr::addr_of_mut!(region_type) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::ProgramMemoryType)?;
        region_type.try_into()
    }
}

/// Compiles the [`Program`] for the given [`Chip`]
pub fn compile_program(program: &Program, chip: &Chip) -> Result<CompilationResult, Error> {
    init_libquil()?;
    let mut compiled_program: quil_program = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_quil.unwrap()(program.0, chip.0, &mut compiled_program);
        crate::handle_libquil_error(err).map_err(Error::CompileQuil)?;
    }

    Ok(CompilationResult {
        program: Program(compiled_program),
        metadata: None,
    })
}

#[derive(Debug, Default, Clone)]
pub struct CompilationMetadata {
    pub final_rewiring: Vec<u32>,
    pub gate_depth: Option<u32>,
    pub multiqubit_gate_depth: Option<u32>,
    pub gate_volume: Option<u32>,
    pub topological_swaps: Option<u32>,
    pub program_duration: Option<f64>,
    pub program_fidelity: Option<f64>,
    pub qpu_runtime_estimation: Option<f64>,
}

macro_rules! get_metadata_field {
    ($metadata_ptr:ident, $field_name:ident, $field_type:ident) => {{
        unsafe {
            let mut var = $field_type::default();
            let mut present = 0;

            paste::paste!(
            let err = [<quilc_compilation_metadata_get_ $field_name>].unwrap()(
                $metadata_ptr,
                std::ptr::addr_of_mut!(var) as *mut _,
                std::ptr::addr_of_mut!(present),
            );
            );
            crate::handle_libquil_error(err).map_err(Error::CompilationMetadata)?;

            if present == 1 {
                Some(var)
            } else {
                None
            }
        }
    }};
}

impl TryFrom<quilc_compilation_metadata> for CompilationMetadata {
    type Error = Error;

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn try_from(value: quilc_compilation_metadata) -> Result<Self, Self::Error> {
        use crate::bindings::*;

        let final_rewiring = unsafe {
            let mut rewiring_ptr: *mut std::ffi::c_uint = std::ptr::null_mut();
            let mut rewiring_len = 0;

            let err = quilc_compilation_metadata_get_final_rewiring.unwrap()(
                value,
                std::ptr::addr_of_mut!(rewiring_ptr) as *mut _,
                std::ptr::addr_of_mut!(rewiring_len) as *mut _,
            );
            crate::handle_libquil_error(err).map_err(Error::CompilationMetadata)?;

            std::slice::from_raw_parts(rewiring_ptr, rewiring_len as usize).to_vec()
        };

        Ok(CompilationMetadata {
            final_rewiring,
            gate_depth: get_metadata_field!(value, gate_depth, u32),
            multiqubit_gate_depth: get_metadata_field!(value, multiqubit_gate_depth, u32),
            gate_volume: get_metadata_field!(value, gate_volume, u32),
            topological_swaps: get_metadata_field!(value, topological_swaps, u32),
            program_duration: get_metadata_field!(value, program_duration, f64),
            program_fidelity: get_metadata_field!(value, program_fidelity, f64),
            qpu_runtime_estimation: get_metadata_field!(value, qpu_runtime_estimation, f64),
        })
    }
}

#[derive(Debug)]
pub struct CompilationResult {
    pub program: Program,
    pub metadata: Option<CompilationMetadata>,
}

/// Compiles the [`Program`] for the given [`Chip`] and restricts
/// the resulting [`Program`] to satisfy "protoquil" constraints
pub fn compile_protoquil(program: &Program, chip: &Chip) -> Result<CompilationResult, Error> {
    init_libquil()?;

    let mut compiled_program: quil_program = std::ptr::null_mut();
    let metadata_ptr: quilc_compilation_metadata = std::ptr::null_mut();

    unsafe {
        let err = quilc_compile_protoquil.unwrap()(
            program.0,
            chip.0,
            std::ptr::addr_of!(metadata_ptr) as *mut _,
            &mut compiled_program,
        );
        crate::handle_libquil_error(err).map_err(Error::CompileProtoquil)?;
    }

    let metadata = metadata_ptr.try_into()?;
    unsafe {
        bindings::lisp_release_handle.unwrap()(metadata_ptr as *mut _);
    }

    Ok(CompilationResult {
        program: Program(compiled_program),
        metadata: Some(metadata),
    })
}

/// Get a fully-connected 2Q [`Chip`]
pub fn get_chip() -> Result<Chip, Error> {
    init_libquil()?;

    let mut chip: chip_specification = std::ptr::null_mut();

    unsafe {
        let err = quilc_build_nq_linear_chip.unwrap()(2, &mut chip);
        crate::handle_libquil_error(err).map_err(Error::BuildNqLinearChip)?;
    }

    Ok(Chip(chip))
}

/// Prints the given [`Program`] to stdout
pub fn print_program(program: &Program) -> Result<(), Error> {
    init_libquil()?;

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
    pauli_terms: Vec<CString>,
    clifford: &Program,
) -> Result<ConjugatePauliByCliffordResult, Error> {
    init_libquil()?;

    unsafe {
        let mut phase = 0;
        let phase_ptr = std::ptr::addr_of_mut!(phase);
        let pauli_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let mut pauli_terms = pauli_terms
            .into_iter()
            .map(CString::into_raw)
            .collect::<Vec<_>>();
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
        for p in pauli_terms {
            let _ = CString::from_raw(p);
        }
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
    seed: Option<i32>,
    interleaver: Option<&Program>,
) -> Result<Vec<Vec<i32>>, Error> {
    init_libquil()?;

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

    let seed_ptr = if let Some(seed) = &seed {
        seed as *const i32
    } else {
        std::ptr::null_mut()
    };

    unsafe {
        let err = quilc_generate_rb_sequence.unwrap()(
            depth,
            qubits,
            gateset.as_mut_ptr() as *mut _,
            gateset.len() as i32,
            seed_ptr as *mut _,
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
        write!(f, "{} ({})", self.version, self.githash)
    }
}

pub fn get_version_info() -> Result<VersionInfo, Error> {
    init_libquil()?;

    unsafe {
        let mut version_info: quilc_version_info = std::ptr::null_mut();
        let err = quilc_get_version_info.unwrap()(&mut version_info);
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        let mut version_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = quilc_version_info_version.unwrap()(
            version_info,
            std::ptr::addr_of_mut!(version_ptr) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        let mut githash_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = quilc_version_info_githash.unwrap()(
            version_info,
            std::ptr::addr_of_mut!(githash_ptr) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::PrintProgram)?;

        let version = get_string_from_pointer_and_free(version_ptr)?;
        let githash = get_string_from_pointer_and_free(githash_ptr)?;

        Ok(VersionInfo { version, githash })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

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
    fn test_program_memory_type() {
        let program = CString::new("DECLARE ro BIT[1]; DECLARE theta REAL[1];")
            .unwrap()
            .try_into()
            .unwrap();

        let expected = MemoryType::Bit;
        let memory_type = program_memory_type(&program, "ro").unwrap();
        assert_eq!(memory_type, expected);

        let expected = MemoryType::Real;
        let memory_type = program_memory_type(&program, "theta").unwrap();
        assert_eq!(memory_type, expected);
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

    fn read_data_file(name: &str) -> String {
        let mut file = File::open(format!(
            "{}/data/{}",
            std::env::var("CARGO_MANIFEST_DIR").unwrap(),
            name
        ))
        .unwrap();
        let mut file_str = String::new();
        file.read_to_string(&mut file_str).unwrap();
        file_str
    }

    #[test]
    fn test_compile_protoquil() {
        let program = new_quil_program();
        let chip = Chip::from_str(&read_data_file("aspen-9-isa.json")).unwrap();
        compile_program(&program, &chip).unwrap();
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
        let x = CString::new("X").unwrap();
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
        let results =
            generate_rb_sequence(3, 1, vec![&phase, &h, &y], Some(42), interleaver).unwrap();
        assert_eq!(results, expected);
    }

    #[test]
    fn test_generate_rb_sequence_without_interleaver() {
        let phase = "PHASE(pi/2) 0".parse().unwrap();
        let h = "H 0".parse().unwrap();
        let y = "Y 0".parse().unwrap();
        let interleaver = None;

        let expected = vec![vec![2, 0, 1], vec![0, 0, 0, 1], vec![0, 1]];
        let results =
            generate_rb_sequence(3, 1, vec![&phase, &h, &y], Some(42), interleaver).unwrap();
        assert_eq!(results, expected);
    }

    #[test]
    fn test_generate_rb_sequence_without_seed() {
        let phase = "PHASE(pi/2) 0".parse().unwrap();
        let h = "H 0".parse().unwrap();
        let y = "Y 0".parse().unwrap();
        let interleaver = None;

        // When no seed is provided, quilc will use SBCL's random state (which we cannot inspect).
        // Thus we cannot check the validity of the results -- only that we don't get an error.
        generate_rb_sequence(3, 1, vec![&phase, &h, &y], None, interleaver).unwrap();
    }
}
