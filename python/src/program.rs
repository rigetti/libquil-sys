use pyo3::prelude::*;

#[pyclass(name = "Program", unsendable)]
pub struct PyProgram(pub(crate) libquil_sys::quilc::Program);

#[pymethods]
impl PyProgram {
    #[new]
    pub fn new(s: &str) -> PyResult<Self> {
        let program: libquil_sys::quilc::Program = s
            .parse()
            .map_err(|err| PyErr::from(crate::RustLibquilQuilcError::from(err)))?;
        Ok(Self(program))
    }
}
