use pyo3::prelude::*;

#[pyclass(name = "Chip", unsendable)]
pub struct PyChip(pub(crate) libquil_sys::quilc::Chip);

#[pymethods]
impl PyChip {
    #[new]
    pub fn new(s: &str) -> PyResult<Self> {
        let chip: libquil_sys::quilc::Chip = s
            .parse()
            .map_err(|err| PyErr::from(crate::RustLibquilQuilcError::from(err)))?;
        Ok(Self(chip))
    }
}
