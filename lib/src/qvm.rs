use std::{collections::HashMap, ffi::CString, fmt::Display};

use crate::{
    bindings::{
        self, qvm_get_version_info, qvm_multishot_addresses, qvm_multishot_addresses_new,
        qvm_multishot_result, qvm_version_info, qvm_version_info_githash, qvm_version_info_version,
    },
    get_string_from_pointer_and_free, handle_libquil_error, init_libquil,
    quilc::{self, program_memory_type},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to get version info: {0}")]
    VersionInfo(String),
    #[error("invalid UTF-8 in version info: {0}")]
    VersionUtf8(#[from] std::str::Utf8Error),
    #[error("failed to serialize to JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
    #[error("failed to convert to CString: {0}")]
    CString(#[from] std::ffi::NulError),
    #[error("failed to perform multishot: {0}")]
    Multishot(String),
    #[error("failed to build multishot addresses: {0}")]
    MultishotAddresses(String),
    #[error("failed to perform multishot measure: {0}")]
    MultishotMeasure(String),
    #[error("failed to perform wavefunction: {0}")]
    Wavefunction(String),
    #[error("failed to perform expectation: {0}")]
    Expectation(String),
    #[error("failed to initialize libquil: {0}")]
    FailedToInitializeLibquil(#[from] crate::Error),
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
        let mut version_info: qvm_version_info = std::ptr::null_mut();
        let err = qvm_get_version_info.unwrap()(&mut version_info);
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        let mut version_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = qvm_version_info_version.unwrap()(
            version_info,
            std::ptr::addr_of_mut!(version_ptr) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        let mut githash_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = qvm_version_info_githash.unwrap()(
            version_info,
            std::ptr::addr_of_mut!(githash_ptr) as *mut _,
        );
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        let version = get_string_from_pointer_and_free(version_ptr)?;
        let githash = get_string_from_pointer_and_free(githash_ptr)?;

        Ok(VersionInfo { version, githash })
    }
}

struct QvmMultishotAddresses {
    addresses: HashMap<String, MultishotAddressRequest>,
    ptr: qvm_multishot_addresses,
}

impl TryFrom<HashMap<String, MultishotAddressRequest>> for QvmMultishotAddresses {
    type Error = Error;

    fn try_from(addresses: HashMap<String, MultishotAddressRequest>) -> Result<Self, Self::Error> {
        let mut addresses_ptr: qvm_multishot_addresses = std::ptr::null_mut();

        unsafe {
            let err = qvm_multishot_addresses_new.unwrap()(&mut addresses_ptr);
            handle_libquil_error(err).map_err(Error::MultishotAddresses)?;
        }

        for (name, address) in &addresses {
            unsafe {
                let name_ptr = CString::new(name.clone())?.into_raw();
                match address {
                    MultishotAddressRequest::All => {
                        let err = bindings::qvm_multishot_addresses_set_all.unwrap()(
                            addresses_ptr,
                            name_ptr,
                        );
                        handle_libquil_error(err).map_err(Error::MultishotAddresses)?;
                    }
                    MultishotAddressRequest::Indices(indices) => {
                        let err = bindings::qvm_multishot_addresses_set.unwrap()(
                            addresses_ptr,
                            name_ptr,
                            indices.to_vec().as_mut_ptr() as *mut _,
                            indices.len() as i32,
                        );
                        handle_libquil_error(err).map_err(Error::MultishotAddresses)?;
                    }
                };
                let _ = CString::from_raw(name_ptr);
            }
        }

        Ok(QvmMultishotAddresses {
            addresses,
            ptr: addresses_ptr,
        })
    }
}

impl IntoIterator for QvmMultishotAddresses {
    type Item = (String, MultishotAddressRequest);

    type IntoIter = std::collections::hash_map::IntoIter<String, MultishotAddressRequest>;

    fn into_iter(self) -> Self::IntoIter {
        self.addresses.into_iter()
    }
}

pub enum MultishotAddressRequest {
    All,
    Indices(Vec<u32>),
}

#[derive(Debug, PartialEq)]
pub enum MultishotAddressData {
    Bit(Vec<Vec<u8>>),
    Octet(Vec<Vec<u8>>),
    Integer(Vec<Vec<u32>>),
    Real(Vec<Vec<f64>>),
}

macro_rules! multishot_get_all {
    ($result:ident, $name:ident, $trial:ident) => {{
        let mut results = std::ptr::null_mut();
        let mut results_len = 0;
        unsafe {
            let err = bindings::qvm_multishot_result_get_all.unwrap()(
                $result,
                $name,
                $trial,
                std::ptr::addr_of_mut!(results) as *mut _,
                std::ptr::addr_of_mut!(results_len) as *mut _,
            );
            crate::handle_libquil_error(err).map_err(Error::Multishot)?;
            (results, results_len)
        }
    }};
}

macro_rules! multishot_get {
    ($result:ident, $name:ident, $trial:ident, $indices:ident, $ty:tt) => {{
        let mut results: Vec<$ty> = vec![$ty::default(); $indices.len()];
        unsafe {
            let err = bindings::qvm_multishot_result_get.unwrap()(
                $result,
                $name,
                $trial,
                results.as_mut_ptr() as *mut _,
            );
            handle_libquil_error(err).map_err(Error::Multishot)?;
            results
        }
    }};
}

/// Execute a program on the QVM and get the measurement results for the provided
/// memory addresses
///
/// The `gate_noise` and `measurement_noise` are 3-tuples (x-noise, y-noise, z-noise)
/// which describe the noise to be applied along each respective axis.
///
/// # Example: specific indices
/// ```
/// use libquil_sys::{quilc, qvm};
/// use std::ffi::CString;
/// use assert2::let_assert;
/// let program = CString::new("DECLARE ro BIT[3]; X 0; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]")
///     .unwrap()
///     .try_into()
///     .unwrap();
/// let addresses = [("ro".to_string(), qvm::MultishotAddressRequest::Indices(vec![0, 2]))].into();
/// let trials = 10;
/// let results = qvm::multishot(&program, addresses, trials, None, None, None).unwrap();
/// // Each of the `trials`-number of elements in `ro` is a
/// // list of the memory address values after execution.
/// let ro = results.get("ro").unwrap();
/// let_assert!(qvm::MultishotAddressData::Bit(ro) = ro);
/// assert_eq!(ro[0], vec![1, 1]);
/// ```
///
/// # Example: all indices
/// ```
/// use libquil_sys::{quilc, qvm};
/// use std::ffi::CString;
/// use assert2::let_assert;
/// let program = CString::new("DECLARE ro BIT[3]; X 0; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]")
///     .unwrap()
///     .try_into()
///     .unwrap();
/// let addresses = [("ro".to_string(), qvm::MultishotAddressRequest::All)].into();
/// let trials = 10;
/// let results = qvm::multishot(&program, addresses, trials, None, None, None).unwrap();
/// // Each of the `trials`-number of elements in `ro` is a
/// // list of the memory address values after execution.
/// let ro = results.get("ro").unwrap();
/// let_assert!(qvm::MultishotAddressData::Bit(ro) = ro);
/// assert_eq!(ro[0], vec![1, 0, 1]);
/// ```
///
/// # Example: with noise
/// ```
/// use libquil_sys::{quilc, qvm};
/// use std::ffi::CString;
/// use assert2::let_assert;
/// let program = CString::new("DECLARE ro BIT[3]; X 0; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]")
///     .unwrap()
///     .try_into()
///     .unwrap();
/// let addresses = [("ro".to_string(), qvm::MultishotAddressRequest::Indices(vec![0, 2]))].into();
/// let trials = 10;
/// let gate_noise = Some((0.1, 0.2, 0.3));
/// let measurement_noise = Some((0.1, 0.2, 0.3));
/// let results = qvm::multishot(&program, addresses, trials, gate_noise, measurement_noise, None).unwrap();
/// // Each of the `trials`-number of elements in `ro` is a
/// // list of the memory address values after execution.
/// let ro = results.get("ro").unwrap();
/// let_assert!(qvm::MultishotAddressData::Bit(ro) = ro);
/// // Because noise has been applied, the results are non-deterministic, and so, unlike the other examples
/// // we cannot make an assertion about the readout data.
/// ```
pub fn multishot(
    program: &quilc::Program,
    addresses: HashMap<String, MultishotAddressRequest>,
    trials: i32,
    gate_noise: Option<(f64, f64, f64)>,
    measurement_noise: Option<(f64, f64, f64)>,
    rng_seed: Option<i64>,
) -> Result<HashMap<String, MultishotAddressData>, Error> {
    let mut multishot = HashMap::new();

    init_libquil()?;
    let addresses: QvmMultishotAddresses = addresses.try_into()?;
    let mut result_ptr: qvm_multishot_result = std::ptr::null_mut();

    let gate_noise = gate_noise.map(|(x, y, z)| vec![x, y, z]);
    let gate_noise_ptr: *mut std::ffi::c_double = if let Some(gate_noise) = &gate_noise {
        gate_noise.as_ptr() as *mut _
    } else {
        std::ptr::null_mut()
    };

    let measurement_noise = measurement_noise.map(|(x, y, z)| vec![x, y, z]);
    let measurement_noise_ptr: *mut std::ffi::c_double =
        if let Some(measurement_noise) = &measurement_noise {
            measurement_noise.as_ptr() as *mut _
        } else {
            std::ptr::null_mut()
        };

    let rng_seed_ptr = if let Some(rng_seed) = &rng_seed {
        rng_seed
    } else {
        std::ptr::null()
    };

    unsafe {
        let err = bindings::qvm_multishot.unwrap()(
            program.0,
            addresses.ptr,
            trials,
            gate_noise_ptr as *mut _,
            measurement_noise_ptr as *mut _,
            rng_seed_ptr as *mut _,
            &mut result_ptr,
        );
        handle_libquil_error(err).map_err(Error::Multishot)?;
    }

    for (name, address) in addresses {
        let address_data_type =
            program_memory_type(program, &name).map_err(|e| Error::Multishot(e.to_string()))?;
        let name_ptr = CString::new(name.clone())?.into_raw();
        let multishot_result =
            multishot
                .entry(name.clone())
                .or_insert_with(|| match address_data_type {
                    quilc::MemoryType::Bit => MultishotAddressData::Bit(vec![]),
                    quilc::MemoryType::Octet => MultishotAddressData::Octet(vec![]),
                    quilc::MemoryType::Integer => MultishotAddressData::Integer(vec![]),
                    quilc::MemoryType::Real => MultishotAddressData::Real(vec![]),
                });

        match address {
            MultishotAddressRequest::All => match multishot_result {
                MultishotAddressData::Bit(result) => {
                    for trial in 0..trials {
                        let (results, len) = multishot_get_all!(result_ptr, name_ptr, trial);

                        unsafe {
                            let results_vec = std::slice::from_raw_parts(results, len).to_vec();
                            result.push(results_vec);
                        }
                    }
                }
                MultishotAddressData::Octet(result) => {
                    for trial in 0..trials {
                        let (results, len) = multishot_get_all!(result_ptr, name_ptr, trial);

                        unsafe {
                            let results_vec = std::slice::from_raw_parts(results, len).to_vec();
                            result.push(results_vec);
                        }
                    }
                }
                MultishotAddressData::Integer(result) => {
                    for trial in 0..trials {
                        let (results, len) = multishot_get_all!(result_ptr, name_ptr, trial);

                        unsafe {
                            let results_vec = std::slice::from_raw_parts(results, len).to_vec();
                            result.push(results_vec);
                        }
                    }
                }
                MultishotAddressData::Real(result) => {
                    for trial in 0..trials {
                        let (results, len) = multishot_get_all!(result_ptr, name_ptr, trial);

                        unsafe {
                            let results_vec = std::slice::from_raw_parts(results, len).to_vec();
                            result.push(results_vec);
                        }
                    }
                }
            },
            MultishotAddressRequest::Indices(indices) => match multishot_result {
                MultishotAddressData::Bit(result) => {
                    for trial in 0..trials {
                        let results = multishot_get!(result_ptr, name_ptr, trial, indices, u8);
                        result.push(results);
                    }
                }
                MultishotAddressData::Octet(result) => {
                    for trial in 0..trials {
                        let results = multishot_get!(result_ptr, name_ptr, trial, indices, u8);
                        result.push(results);
                    }
                }
                MultishotAddressData::Integer(result) => {
                    for trial in 0..trials {
                        let results = multishot_get!(result_ptr, name_ptr, trial, indices, u32);
                        result.push(results);
                    }
                }
                MultishotAddressData::Real(result) => {
                    for trial in 0..trials {
                        let results = multishot_get!(result_ptr, name_ptr, trial, indices, f64);
                        result.push(results);
                    }
                }
            },
        }

        unsafe {
            let _ = CString::from_raw(name_ptr);
        }
    }

    unsafe {
        bindings::lisp_release_handle.unwrap()(result_ptr as *mut _);
    }

    Ok(multishot)
}

/// Execute a program on the QVM and get the measurement results for the provided
/// qubits
///
/// # Example
/// ```
/// use libquil_sys::{quilc, qvm};
/// use std::ffi::CString;
/// let program = CString::new("X 0; H 2")
///     .unwrap()
///     .try_into()
///     .unwrap();
/// let qubits = vec![0, 2];
/// let trials = 10;
/// let results = qvm::multishot_measure(&program, &qubits, trials, None).unwrap();
/// for (trial, measurements) in results.iter().enumerate() {
///     println!("Trial {trial}: [q0={}, q1={}]", measurements[0], measurements[1]);
/// }
/// ```
pub fn multishot_measure(
    program: &quilc::Program,
    qubits: &[i32],
    trials: i32,
    rng_seed: Option<i64>,
) -> Result<Vec<Vec<i32>>, Error> {
    init_libquil()?;

    // NOTE(mgsk): There might be a way for this to be a Vec<Vec<i32>>
    // which would exactly match our return type. In practice, however,
    // that type always resulted in an error "SIGSEGV: invalid memory
    // reference" coming from the lisp image when trying to access
    // the data after lisp had populated it.
    let mut results = vec![0; qubits.len() * trials as usize];
    let mut qubits = qubits.to_vec();
    let rng_seed_ptr = if let Some(rng_seed) = &rng_seed {
        rng_seed
    } else {
        std::ptr::null()
    };

    unsafe {
        let err = bindings::qvm_multishot_measure.unwrap()(
            program.0,
            qubits.as_mut_ptr() as *mut _,
            qubits.len() as i32,
            trials,
            rng_seed_ptr as *mut _,
            results.as_mut_ptr() as *mut _,
        );
        handle_libquil_error(err).map_err(Error::MultishotMeasure)?;
    }

    Ok(results.chunks(qubits.len()).map(Into::into).collect())
}

/// Calculate the wavefunction produced by `program`.
///
/// The result is a vector of complex numbers of length `2*n_qubits`. See [`probabilities`]
/// for a description of the interpretation of the vector indices.
pub fn wavefunction(
    program: &quilc::Program,
    rng_seed: Option<i64>,
) -> Result<Vec<num_complex::Complex64>, Error> {
    init_libquil()?;

    // let mut wavefunction = vec![0.0; 2 * 2u32.pow(n_qubits) as usize];
    // let wavefunction
    let mut results: *mut std::ffi::c_double = std::ptr::null_mut();
    let mut results_len = 0;
    let rng_seed_ptr = if let Some(rng_seed) = &rng_seed {
        rng_seed
    } else {
        std::ptr::null()
    };

    unsafe {
        let err = bindings::qvm_wavefunction.unwrap()(
            program.0,
            rng_seed_ptr as *mut _,
            std::ptr::addr_of_mut!(results) as *mut _,
            std::ptr::addr_of_mut!(results_len) as *mut _,
        );
        handle_libquil_error(err).map_err(Error::Wavefunction)?;
        let wavefunction = std::slice::from_raw_parts(results, results_len);
        Ok(wavefunction
            .chunks(2)
            .map(|c| num_complex::Complex::new(c[0], c[1]))
            .collect::<Vec<_>>())
    }
}

/// Calculate the probabilities for each quantum state.
///
/// The result is a vector `v` of length `2^n_qubits` where `v[i]` is the probability
/// of finding the quantum state in `|i>`. For example, `v[2]` is the probability
/// of finding the quantum state in `|10>`; `v[5]` the probability of `|101>`; etc.
pub fn probabilities(
    program: &quilc::Program,
    n_qubits: u32,
    rng_seed: Option<i64>,
) -> Result<Vec<f64>, Error> {
    init_libquil()?;

    let mut probabilities = vec![0.0; 2u32.pow(n_qubits) as usize];
    let rng_seed_ptr = if let Some(rng_seed) = &rng_seed {
        rng_seed
    } else {
        std::ptr::null()
    };

    unsafe {
        let err = bindings::qvm_probabilities.unwrap()(
            program.0,
            rng_seed_ptr as *mut _,
            probabilities.as_mut_ptr() as *mut _,
        );
        handle_libquil_error(err).map_err(Error::Wavefunction)?;
    }

    Ok(probabilities)
}

/// Calculate the expectation value `<O|P|O>` for each operator `O` in `program`.
pub fn expectation(
    program: &quilc::Program,
    operators: Vec<&quilc::Program>,
    rng_seed: Option<i64>,
) -> Result<Vec<f64>, Error> {
    init_libquil()?;

    let rng_seed_ptr = if let Some(rng_seed) = &rng_seed {
        rng_seed
    } else {
        std::ptr::null()
    };

    unsafe {
        let mut expectations = vec![0.0; operators.len()];
        let err = bindings::qvm_expectation.unwrap()(
            program.0,
            operators
                .iter()
                .map(|p| p.0)
                .collect::<Vec<_>>()
                .as_mut_ptr() as *mut _,
            operators.len() as i32,
            rng_seed_ptr as *mut _,
            expectations.as_mut_ptr() as *mut _,
        );
        handle_libquil_error(err).map_err(Error::Expectation)?;
        Ok(expectations)
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, ffi::CString};

    use assert2::let_assert;

    use crate::{
        quilc,
        qvm::{expectation, probabilities},
    };

    use super::{
        get_version_info, multishot, multishot_measure, wavefunction, MultishotAddressData,
        MultishotAddressRequest,
    };

    #[test]
    fn test_multishot_all_memory_types() {
        let program: quilc::Program =
            CString::new("DECLARE ro BIT[2]; DECLARE count INTEGER[1]; DECLARE theta REAL[2]; MOVE ro[0] 1; MOVE count[0] 10; MOVE theta[1] 4.2;")
                .unwrap()
                .try_into()
                .unwrap();

        let addresses = [
            (
                "ro".to_string(),
                MultishotAddressRequest::Indices(vec![0, 1]),
            ),
            ("count".to_string(), MultishotAddressRequest::All),
            (
                "theta".to_string(),
                MultishotAddressRequest::Indices(vec![1]),
            ),
        ]
        .into();
        let expected: HashMap<String, MultishotAddressData> = [
            (
                "ro".to_string(),
                MultishotAddressData::Bit(vec![vec![1, 0]]),
            ),
            (
                "count".to_string(),
                MultishotAddressData::Integer(vec![vec![10]]),
            ),
            (
                "theta".to_string(),
                MultishotAddressData::Real(vec![vec![4.2]]),
            ),
        ]
        .into();
        let results = multishot(&program, addresses, 1, None, None, None).unwrap();
        assert_eq!(results, expected);
    }

    #[test]
    fn test_pyquil_multishot_failure() {
        let program: quilc::Program = CString::new(
            r#"DECLARE ro BIT[3]
X 0
X 1
X 2
MEASURE 0 ro[0]
MEASURE 1 ro[1]
MEASURE 2 ro[2]
"#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        let addresses = [("ro".to_string(), MultishotAddressRequest::All)].into();
        let results = multishot(&program, addresses, 1, None, None, None).unwrap();
        let expected = [(
            "ro".to_string(),
            MultishotAddressData::Bit(vec![vec![1, 1, 1]]),
        )]
        .into();
        assert_eq!(results, expected);
    }

    #[test]
    fn test_multishot_bell_state() {
        let program: quilc::Program =
            CString::new("DECLARE ro BIT[2]; H 0; CNOT 0 1; MEASURE 0 ro[0]; MEASURE 1 ro[1]")
                .unwrap()
                .try_into()
                .unwrap();

        let addresses = [(
            "ro".to_string(),
            MultishotAddressRequest::Indices(vec![0, 1]),
        )]
        .into();
        let results = multishot(&program, addresses, 2, None, None, None).unwrap();
        for (name, result) in results {
            let_assert!(MultishotAddressData::Bit(result) = result);
            for trial in result {
                let first = trial[0];
                assert!(
                    trial.iter().all(|&v| v == first),
                    "expected multishot trial for {name} to have equal elements ({trial:?})"
                );
            }
        }
    }

    #[test]
    fn test_multishot_deterministic() {
        let program: quilc::Program = CString::new(
            "DECLARE ro BIT[3]; X 0; I 1; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let expected = vec![1, 0, 1];

        let addresses = [(
            "ro".to_string(),
            MultishotAddressRequest::Indices(vec![0, 1, 2]),
        )]
        .into();
        let results = multishot(&program, addresses, 2, None, None, None).unwrap();
        for result in results.values() {
            let_assert!(MultishotAddressData::Bit(result) = result);
            for trial in result {
                assert_eq!(trial, &expected);
            }
        }
    }

    #[test]
    fn test_multishot_with_noise() {
        let program: quilc::Program = CString::new(
            "DECLARE ro BIT[3]; X 0; I 1; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]",
        )
        .unwrap()
        .try_into()
        .unwrap();

        let gate_noise = Some((0.1, 0.0, 0.0));
        let measurement_noise = Some((0.0, 0.0, 0.1));
        let addresses = [(
            "ro".to_string(),
            MultishotAddressRequest::Indices(vec![0, 1, 2]),
        )]
        .into();
        multishot(&program, addresses, 2, gate_noise, measurement_noise, None).unwrap();
        // Cannot assert an expected result because noise was applied
    }

    #[test]
    fn test_multishot_with_kraus_operator() {
        let program: quilc::Program = CString::new(
            r#"X 1
PRAGMA ADD-KRAUS X 1 "(1.0 0.0 0.0 1.0)"
PRAGMA READOUT-POVM 1 "(0.9 0.2 0.1 0.8)"
DECLARE ro BIT[1]
MEASURE 1 ro[0]"#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        let addresses = [("ro".to_string(), MultishotAddressRequest::Indices(vec![0]))].into();

        multishot(&program, addresses, 2, None, None, None).unwrap();
        // Cannot assert an expected result because noise was applied
    }

    #[test]
    fn test_multishot_deterministic_all_indices() {
        let program: quilc::Program = CString::new(
            "DECLARE ro BIT[3]; X 0; I 1; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[1]; MEASURE 2 ro[2]",
        )
        .unwrap()
        .try_into()
        .unwrap();
        let expected = vec![1, 0, 1];

        let addresses = [("ro".to_string(), MultishotAddressRequest::All)].into();
        let results = multishot(&program, addresses, 2, None, None, None).unwrap();
        for result in results.values() {
            let_assert!(MultishotAddressData::Bit(result) = result);
            for trial in result {
                assert_eq!(trial, &expected);
            }
        }
    }

    #[test]
    fn test_multishot_measure_deterministic() {
        let program: quilc::Program = CString::new("X 0; I 2;").unwrap().try_into().unwrap();
        let trials = 10;
        let expected = vec![1, 0];

        let qubits = &[0, 2];
        let results = multishot_measure(&program, qubits, trials, None).unwrap();
        for result in results {
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_wavefunction() {
        let C0 = num_complex::Complex::new(0.0, 0.0);
        let program: quilc::Program = CString::new("X 0; I 1;").unwrap().try_into().unwrap();
        let mut expected = vec![C0; 4];
        expected[1] = num_complex::Complex::new(1.0, 0.0);

        let results = wavefunction(&program, None).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_probabilities() {
        let program: quilc::Program = CString::new("X 0; I 1;").unwrap().try_into().unwrap();
        let mut expected = vec![0.0; 4];
        expected[1] = 1.0;

        let results = probabilities(&program, 2, None).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_expectation() {
        let i: quilc::Program = CString::new("I 0").unwrap().try_into().unwrap();
        let z: quilc::Program = CString::new("Z 0").unwrap().try_into().unwrap();
        let x: quilc::Program = CString::new("X 0").unwrap().try_into().unwrap();
        let operators = vec![&z, &x];
        let expected = vec![1.0, 0.0];

        let results = expectation(&i, operators, None).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_get_version_info() {
        get_version_info().unwrap();
    }
}
