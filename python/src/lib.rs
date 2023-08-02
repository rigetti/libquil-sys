use pyo3::prelude::*;
use pyo3::types::PyString;

use rigetti_pyo3::{
    create_init_submodule, py_wrap_error, py_wrap_struct, wrap_error, PyWrapper, ToPythonError,
};

create_init_submodule! {
    classes: [PyChip, PyProgram],
    funcs: [compile],
}

wrap_error! {
    RustLibquilError(libquil_sys::Error)
}

py_wrap_error!(
    libquil_py,
    RustLibquilError,
    PyLibquilError,
    pyo3::exceptions::PyException
);

#[derive(Clone)]
pub struct Chip(libquil_sys::Chip);

py_wrap_struct! {
    PyChip(Chip) as "Chip" {
        py -> rs {
            str: Py<PyString> => Chip {
                let s = str.as_ref(py).to_str()?;
                let lc: libquil_sys::Chip = s.parse().map_err(|err: libquil_sys::Error| RustLibquilError::from(err).to_py_err())?;
                let c: Chip = Chip(lc);
                Ok::<_, PyErr>(c)
            }
        },
    }
}

#[derive(Clone)]
pub struct Program(libquil_sys::Program);

py_wrap_struct! {
    PyProgram(Program) as "Program" {
        py -> rs {
            str: Py<PyString> => Chip {
                let s = str.as_ref(py).to_str()?;
                let program = s.parse().map_err(|err: libquil_sys::Error| RustLibquilError::from(err).to_py_err())?;
                Ok::<_, PyErr>(Program(program))
            }
        },
    }
}

#[pymodule]
fn libquil_py(py: Python<'_>, m: &PyModule) -> PyResult<()> {
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

#[pyfunction]
pub fn compile(program: PyProgram, chip: PyChip) -> PyResult<String> {
    let compiled_program =
        libquil_sys::compile_program(&program.into_inner().0, &chip.into_inner().0)
            .map_err(|e| RustLibquilError::from(e).to_py_err())?;
    compiled_program
        .to_string()
        .map_err(|e| RustLibquilError::from(e).to_py_err())
}
