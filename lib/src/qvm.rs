use std::{
    collections::HashMap,
    ffi::{CStr, CString},
};

use crate::{
    bindings::{
        self, qvm_get_version_info, qvm_multishot_addresses, qvm_multishot_addresses_new,
        qvm_multishot_result, qvm_version_info, qvm_version_info_githash, qvm_version_info_version,
    },
    handle_libquil_error, init_libquil, quilc,
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
}

#[derive(Debug)]
pub struct VersionInfo {
    pub version: String,
    pub githash: String,
}

pub fn get_version_info() -> Result<VersionInfo, Error> {
    init_libquil();

    unsafe {
        let mut version_info: qvm_version_info = std::ptr::null_mut();
        let err = qvm_get_version_info.unwrap()(&mut version_info);
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        let mut version_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = qvm_version_info_version.unwrap()(version_info, &mut version_ptr);
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        let mut githash_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let err = qvm_version_info_githash.unwrap()(version_info, &mut githash_ptr);
        crate::handle_libquil_error(err).map_err(Error::VersionInfo)?;

        Ok(VersionInfo {
            version: CStr::from_ptr(version_ptr).to_str()?.to_string(),
            githash: CStr::from_ptr(githash_ptr).to_str()?.to_string(),
        })
    }
}

struct QvmMultishotAddresses {
    addresses: HashMap<String, Vec<u32>>,
    ptr: qvm_multishot_addresses,
}

impl TryFrom<HashMap<String, Vec<u32>>> for QvmMultishotAddresses {
    type Error = Error;

    fn try_from(addresses: HashMap<String, Vec<u32>>) -> Result<Self, Self::Error> {
        let mut addresses_ptr: qvm_multishot_addresses = std::ptr::null_mut();

        unsafe {
            let err = qvm_multishot_addresses_new.unwrap()(&mut addresses_ptr);
            handle_libquil_error(err).map_err(Error::MultishotAddresses)?;
        }

        for (name, indices) in &addresses {
            unsafe {
                let name_ptr = CString::new(name.clone())?.into_raw();
                let err = bindings::qvm_multishot_addresses_set.unwrap()(
                    addresses_ptr,
                    name_ptr,
                    indices.to_vec().as_mut_ptr() as *mut _,
                    indices.len() as i32,
                );
                handle_libquil_error(err).map_err(Error::MultishotAddresses)?;
            }
        }

        Ok(QvmMultishotAddresses {
            addresses,
            ptr: addresses_ptr,
        })
    }
}

impl IntoIterator for QvmMultishotAddresses {
    type Item = (String, Vec<u32>);

    type IntoIter = std::collections::hash_map::IntoIter<String, Vec<u32>>;

    fn into_iter(self) -> Self::IntoIter {
        self.addresses.into_iter()
    }
}

/// Execute a program on the QVM and get the measurement results for the provided
/// memory addresses
///
/// # Example
/// ```
/// use libquil_sys::{quilc, qvm};
/// use std::ffi::CString;
/// let program = CString::new("DECLARE ro BIT[3]; X 0; X 2; MEASURE 0 ro[0]; MEASURE 1 ro[2]")
///     .unwrap()
///     .try_into()
///     .unwrap();
/// let addresses = [("ro".to_string(), vec![0, 2])].into();
/// let trials = 10;
/// let results = qvm::multishot(&program, addresses, trials).unwrap();
/// // Each of the `trials`-number of elements in `ro` is a
/// // list of the memory address values after execution.
/// let ro = results.get("ro").unwrap();
/// println!("{ro:?}");
/// ```
pub fn multishot(
    program: &quilc::Program,
    addresses: HashMap<String, Vec<u32>>,
    trials: i32,
) -> Result<HashMap<String, Vec<Vec<u32>>>, Error> {
    let mut multishot = HashMap::new();

    init_libquil();
    let addresses: QvmMultishotAddresses = addresses.try_into()?;
    let mut result_ptr: qvm_multishot_result = std::ptr::null_mut();

    unsafe {
        let err =
            bindings::qvm_multishot.unwrap()(program.0, addresses.ptr, trials, &mut result_ptr);
        handle_libquil_error(err).map_err(Error::Multishot)?;
    }

    for (name, indices) in addresses {
        let name_ptr = CString::new(name.clone())?.into_raw();
        let multishot_result: &mut Vec<Vec<u32>> = multishot.entry(name).or_default();
        for trial in 0..trials {
            unsafe {
                let mut results: Vec<u32> = vec![0; indices.len()];
                let err = bindings::qvm_multishot_result_get.unwrap()(
                    result_ptr,
                    name_ptr,
                    trial,
                    results.as_mut_ptr() as *mut _,
                );
                handle_libquil_error(err).map_err(Error::Multishot)?;
                multishot_result.push(results);
            }
        }
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
/// let results = qvm::multishot_measure(&program, &qubits, trials).unwrap();
/// for (trial, measurements) in results.iter().enumerate() {
///     println!("Trial {trial}: [q0={}, q1={}]", measurements[0], measurements[1]);
/// }
/// ```
pub fn multishot_measure(
    program: &quilc::Program,
    qubits: &[i32],
    trials: i32,
) -> Result<Vec<Vec<i32>>, Error> {
    init_libquil();

    // NOTE(mgsk): There might be a way for this to be a Vec<Vec<i32>>
    // which would exactly match our return type. In practice, however,
    // that type always resulted in an error "SIGSEGV: invalid memory
    // reference" coming from the lisp image when trying to access
    // the data after lisp had populated it.
    let mut results = vec![0; qubits.len() * trials as usize];
    let mut qubits = qubits.to_vec();

    unsafe {
        let err = bindings::qvm_multishot_measure.unwrap()(
            program.0,
            qubits.as_mut_ptr() as *mut _,
            qubits.len() as i32,
            trials,
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
    n_qubits: u32,
) -> Result<Vec<num_complex::Complex64>, Error> {
    init_libquil();

    let mut wavefunction = vec![0.0; 2 * 2u32.pow(n_qubits) as usize];

    unsafe {
        let err =
            bindings::qvm_wavefunction.unwrap()(program.0, wavefunction.as_mut_ptr() as *mut _);
        handle_libquil_error(err).map_err(Error::Wavefunction)?;
    }

    Ok(wavefunction
        .chunks(2)
        .map(|c| num_complex::Complex::new(c[0], c[1]))
        .collect::<Vec<_>>())
}

/// Calculate the probabilities for each quantum state.
///
/// The result is a vector `v` of length `2^n_qubits` where `v[i]` is the probability
/// of finding the quantum state in `|i>`. For example, `v[2]` is the probability
/// of finding the quantum state in `|10>`; `v[5]` the probability of `|101>`; etc.
pub fn probabilities(program: &quilc::Program, n_qubits: u32) -> Result<Vec<f64>, Error> {
    init_libquil();

    let mut probabilities = vec![0.0; 2u32.pow(n_qubits) as usize];

    unsafe {
        let err =
            bindings::qvm_probabilities.unwrap()(program.0, probabilities.as_mut_ptr() as *mut _);
        handle_libquil_error(err).map_err(Error::Wavefunction)?;
    }

    Ok(probabilities)
}

/// Calculate the expectation value `<O|P|O>` for each operator `O` in `program`.
pub fn expectation(
    program: &quilc::Program,
    operators: Vec<&quilc::Program>,
) -> Result<Vec<f64>, Error> {
    init_libquil();

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
            expectations.as_mut_ptr() as *mut _,
        );
        handle_libquil_error(err).map_err(Error::Expectation)?;
        Ok(expectations)
    }
}

#[cfg(test)]
mod test {
    use std::ffi::CString;

    use crate::{
        quilc,
        qvm::{expectation, probabilities},
    };

    use super::{get_version_info, multishot, multishot_measure, wavefunction};

    #[test]
    fn test_multishot_bell_state() {
        let program: quilc::Program =
            CString::new("DECLARE ro BIT[2]; H 0; CNOT 0 1; MEASURE 0 ro[0]; MEASURE 1 ro[1]")
                .unwrap()
                .try_into()
                .unwrap();

        let addresses = [("ro".to_string(), vec![0, 1])].into();
        let results = multishot(&program, addresses, 2).unwrap();
        for (name, result) in results {
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

        let addresses = [("ro".to_string(), vec![0, 1, 2])].into();
        let results = multishot(&program, addresses, 2).unwrap();
        for result in results.values() {
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
        let results = multishot_measure(&program, qubits, trials).unwrap();
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

        let results = wavefunction(&program, 2).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_probabilities() {
        let program: quilc::Program = CString::new("X 0; I 1;").unwrap().try_into().unwrap();
        let mut expected = vec![0.0; 4];
        expected[1] = 1.0;

        let results = probabilities(&program, 2).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_expectation() {
        let i: quilc::Program = CString::new("I 0").unwrap().try_into().unwrap();
        let z: quilc::Program = CString::new("Z 0").unwrap().try_into().unwrap();
        let x: quilc::Program = CString::new("X 0").unwrap().try_into().unwrap();
        let operators = vec![&z, &x];
        let expected = vec![1.0, 0.0];

        let results = expectation(&i, operators).unwrap();
        assert_eq!(results, expected)
    }

    #[test]
    fn test_get_version_info() {
        get_version_info().unwrap();
    }
}
