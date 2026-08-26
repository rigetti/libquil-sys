mod chip;
mod compile;
mod program;

use pyo3::prelude::*;

use rigetti_pyo3::{create_init_submodule, exception};

/// Newtype around [`libquil_sys::quilc::Error`] so it can be converted into a
/// Python exception (the orphan rule forbids implementing `From` for the
/// foreign error type directly).
#[derive(Debug)]
pub struct RustLibquilQuilcError(libquil_sys::quilc::Error);

impl From<libquil_sys::quilc::Error> for RustLibquilQuilcError {
    fn from(err: libquil_sys::quilc::Error) -> Self {
        Self(err)
    }
}

impl std::fmt::Display for RustLibquilQuilcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for RustLibquilQuilcError {}

exception!(
    RustLibquilQuilcError,
    "libquil",
    PyLibquilError,
    pyo3::exceptions::PyException
);

#[pymodule]
fn libquil(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    init_submodule("libquil", py, m)?;
    Ok(())
}

create_init_submodule! {
    classes: [chip::PyChip, program::PyProgram, compile::PyCompileOptions],
    errors: [PyLibquilError],
    funcs: [compile::compile],
}
