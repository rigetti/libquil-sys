use std::{collections::HashMap, ffi::CString};

use crate::{
    bindings::{self, qvm_multishot_result},
    init_libquil, quilc,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to serialize to JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
    #[error("failed to convert to CString: {0}")]
    CString(#[from] std::ffi::NulError),
    #[error("failed to perform multishot: {0}")]
    Multishot(String),
    #[error("failed to perform multishot measure: {0}")]
    MultishotMeasure(String),
    #[error("failed to perform wavefunction: {0}")]
    Wavefunction(String),
}

pub fn get_version_info() {
    todo!()
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
    addresses: HashMap<String, Vec<usize>>,
    trials: i32,
) -> Result<HashMap<String, Vec<Vec<u32>>>, Error> {
    let mut multishot = HashMap::new();

    init_libquil();
    let addresses_json = serde_json::to_string(&addresses)?;
    let mut result_ptr: qvm_multishot_result = std::ptr::null_mut();
    let addresses_ptr = CString::new(addresses_json)?.into_raw();

    unsafe {
        let err =
            bindings::qvm_multishot.unwrap()(program.0, addresses_ptr, trials, &mut result_ptr);
        crate::handle_libquil_error(err).map_err(Error::Multishot)?;
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
                crate::handle_libquil_error(err).map_err(Error::Multishot)?;
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
        crate::handle_libquil_error(err).map_err(Error::MultishotMeasure)?;
    }

    Ok(results.chunks(qubits.len()).map(Into::into).collect())
}

pub fn wavefunction(program: &quilc::Program, n_qubits: usize) -> Result<Vec<f64>, Error> {
    init_libquil();

    let mut wavefunction = vec![0.0; 2u32.pow(n_qubits as u32) as usize];

    unsafe {
        let err =
            bindings::qvm_wavefunction.unwrap()(program.0, wavefunction.as_mut_ptr() as *mut _);
        crate::handle_libquil_error(err).map_err(Error::Wavefunction)?;
    }

    Ok(wavefunction)
}

pub fn probabilities() {
    todo!()
}

pub fn expectation() {
    todo!()
}

#[cfg(test)]
mod test {
    use std::ffi::CString;

    use crate::quilc;

    use super::{multishot, multishot_measure, wavefunction};

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
        let program: quilc::Program =
            CString::new("DECLARE ro BIT[2]; X 0; I 1; MEASURE 0 ro[0]; MEASURE 1 ro[1]")
                .unwrap()
                .try_into()
                .unwrap();
        let expected = vec![1, 0];

        let addresses = [("ro".to_string(), vec![0, 1])].into();
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
    fn test_multishot_wavefunction() {
        let program: quilc::Program = CString::new("X 0; I 1;").unwrap().try_into().unwrap();
        let expected = vec![1.0, 0.0];

        let results = wavefunction(&program, 2).unwrap();

        assert_eq!(results, expected)
    }
}
