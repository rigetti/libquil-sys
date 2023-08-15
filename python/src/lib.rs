mod chip;
mod compile;
mod program;

use pyo3::prelude::*;

use rigetti_pyo3::{create_init_submodule, py_wrap_error, wrap_error};

wrap_error! {
    RustLibquilError(libquil_sys::Error)
}

py_wrap_error!(
    libquil,
    RustLibquilError,
    PyLibquilError,
    pyo3::exceptions::PyException
);

#[pymodule]
fn libquil(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    init_submodule("libquil_py", py, m)?;
    Ok(())
}

create_init_submodule! {
    classes: [chip::PyChip, program::PyProgram, compile::PyCompileOptions],
    funcs: [compile::compile],
}
