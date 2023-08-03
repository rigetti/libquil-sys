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
    unsafe {
        // The library built by maturin does link to libquilc, but
        // the linker does not make the libquilc symbols available
        // to the lisp image. To get around that, we load it here
        // with the `RTLD_GLOBAL` flag which makes symbols available
        // to the whole process.
        libloading::os::unix::Library::open(
            Some("libquilc.so"),
            libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL,
        )
        .unwrap();
    }
    init_submodule("libquil_py", py, m)?;
    Ok(())
}

create_init_submodule! {
    classes: [chip::PyChip, program::PyProgram, compile::PyCompileOptions],
    funcs: [compile::compile],
}
