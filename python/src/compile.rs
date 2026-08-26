use pyo3::{exceptions::PyValueError, prelude::*};

#[pyclass(name = "CompileOptions", skip_from_py_object)]
#[derive(Debug, Clone, Default)]
pub struct PyCompileOptions {
    pub protoquil: Option<bool>,
}

#[pymethods]
impl PyCompileOptions {
    #[new]
    #[pyo3(signature = (protoquil=None))]
    pub fn new(protoquil: Option<bool>) -> Self {
        Self { protoquil }
    }

    #[getter]
    pub fn get_protoquil(&self) -> Option<bool> {
        self.protoquil
    }

    #[setter]
    pub fn set_protoquil(&mut self, protoquil: Option<bool>) {
        self.protoquil = protoquil;
    }
}

#[pyclass(name = "CompilationMetadata", skip_from_py_object)]
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
    let protoquil = options.and_then(|e| e.protoquil);

    let compilation_result = if let Some(true) = protoquil {
        libquil_sys::quilc::compile_protoquil(&program.0, &chip.0)
            .map_err(|e| PyErr::from(crate::RustLibquilQuilcError::from(e)))?
    } else {
        libquil_sys::quilc::compile_program(&program.0, &chip.0)
            .map_err(|e| PyErr::from(crate::RustLibquilQuilcError::from(e)))?
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
