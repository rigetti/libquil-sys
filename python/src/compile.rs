use pyo3::{exceptions::PyValueError, prelude::*, types::PyBool};
use rigetti_pyo3::{py_wrap_data_struct, PyWrapper, ToPythonError};

#[derive(Clone)]
pub struct CompileOptions {
    pub protoquil: Option<bool>,
}

py_wrap_data_struct! {
    PyCompileOptions(CompileOptions) as "CompileOptions" {
        protoquil: Option<bool> => Option<Py<PyBool>>
    }
}

#[pymethods]
impl PyCompileOptions {
    #[new]
    pub fn new(py: Python<'_>, protoquil: Option<Py<PyBool>>) -> PyResult<Self> {
        let protoquil = protoquil.map(|p| p.is_true(py)).transpose()?;
        Ok(Self(CompileOptions { protoquil }))
    }
}

#[pyclass(name = "CompilationMetadata")]
#[derive(Debug, Clone)]
pub struct PyCompilationMetadata(libquil_sys::quilc::CompilationMetadata);

#[pymethods]
impl PyCompilationMetadata {
    #[getter]
    pub fn get_final_rewiring(&self) -> Vec<u32> {
        self.0.final_rewiring.clone()
    }

    #[getter]
    pub fn get_gate_depth(&self) -> Option<u32> {
        self.0.gate_depth
    }

    #[getter]
    pub fn get_multiqubit_gate_depth(&self) -> Option<u32> {
        self.0.multiqubit_gate_depth
    }

    #[getter]
    pub fn get_gate_volume(&self) -> Option<u32> {
        self.0.gate_volume
    }

    #[getter]
    pub fn get_topological_swaps(&self) -> Option<u32> {
        self.0.topological_swaps
    }

    #[getter]
    pub fn get_program_duration(&self) -> Option<f64> {
        self.0.program_duration
    }

    #[getter]
    pub fn get_program_fidelity(&self) -> Option<f64> {
        self.0.program_fidelity
    }

    #[getter]
    pub fn get_qpu_runtime_estimation(&self) -> Option<f64> {
        self.0.qpu_runtime_estimation
    }
}

#[pyclass(name = "CompilationResult")]
#[derive(Debug)]
pub struct PyCompilationResult {
    program: String,
    metadata: Option<PyCompilationMetadata>,
}

#[pymethods]
impl PyCompilationResult {
    #[getter]
    pub fn get_program(&self) -> String {
        self.program.clone()
    }

    #[getter]
    pub fn get_metadata(&self) -> Option<PyCompilationMetadata> {
        self.metadata.clone()
    }
}

#[pyfunction]
pub fn compile(
    program: &crate::program::PyProgram,
    chip: &crate::chip::PyChip,
    options: Option<&PyCompileOptions>,
) -> PyResult<PyCompilationResult> {
    let protoquil = options.and_then(|e| e.as_inner().protoquil);

    let compilation_result = if let Some(true) = protoquil {
        libquil_sys::quilc::compile_protoquil(&program.as_inner().0, &chip.as_inner().0)
            .map_err(|e| crate::RustLibquilQuilcError::from(e).to_py_err())?
    } else {
        libquil_sys::quilc::compile_program(&program.as_inner().0, &chip.as_inner().0)
            .map_err(|e| crate::RustLibquilQuilcError::from(e).to_py_err())?
    };

    let metadata = compilation_result.metadata.map(PyCompilationMetadata);

    Ok(PyCompilationResult {
        program: compilation_result
            .program
            .to_string()
            .map_err(|e| PyValueError::new_err(format!("failed to stringify program: {e}")))?,
        metadata,
    })
}
