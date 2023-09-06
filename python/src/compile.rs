use pyo3::{prelude::*, types::PyBool};

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

#[pyfunction]
pub fn compile(
    program: crate::program::PyProgram,
    chip: crate::chip::PyChip,
    options: Option<PyCompileOptions>,
) -> PyResult<String> {
    let protoquil = options.and_then(|e| e.into_inner().protoquil);

    let compiled_program = if let Some(true) = protoquil {
        libquil_sys::quilc::compile_protoquil(&program.into_inner().0, &chip.into_inner().0)
            .map_err(|e| crate::RustLibquilQuilcError::from(e).to_py_err())?
    } else {
        libquil_sys::quilc::compile_program(&program.into_inner().0, &chip.into_inner().0)
            .map_err(|e| crate::RustLibquilQuilcError::from(e).to_py_err())?
    };

    compiled_program
        .to_string()
        .map_err(|e| crate::RustLibquilQuilcError::from(e).to_py_err())
}
