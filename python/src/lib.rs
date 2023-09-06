mod chip;
mod compile;
mod program;

use pyo3::prelude::*;

use rigetti_pyo3::{create_init_submodule, py_wrap_error, wrap_error};

wrap_error! {
    RustLibquilQuilcError(libquil_sys::quilc::Error)
}

py_wrap_error!(
    libquil,
    RustLibquilQuilcError,
    PyLibquilError,
    pyo3::exceptions::PyException
);

#[pymodule]
fn libquil(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    init_submodule("libquil", py, m)?;
    Ok(())
}

create_init_submodule! {
    classes: [chip::PyChip, program::PyProgram, compile::PyCompileOptions],
    funcs: [compile::compile],
}
