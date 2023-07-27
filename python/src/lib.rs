use pyo3::prelude::*;
use pyo3::types::PyString;
use std::ffi::CString;

use rigetti_pyo3::{create_init_submodule, py_wrap_struct, PyWrapper};

create_init_submodule! {
    classes: [PyChip, PyProgram],
    funcs: [compile],
}

#[derive(Clone)]
pub struct Chip(libquil_sys::Chip);

py_wrap_struct! {
    PyChip(Chip) as "Chip" {
        py -> rs {
            str: Py<PyString> => Chip {
                let cstr = CString::new(str.as_ref(py).to_str()?)?;
                Ok::<_, PyErr>(Chip(cstr.into()))
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
                let cstr = CString::new(str.as_ref(py).to_str()?)?;
                dbg!(&cstr);
                Ok::<_, PyErr>(Program(cstr.into()))
            }
        },
    }
}

#[pymodule]
fn libquil(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    init_submodule("libquil", py, m)?;
    Ok(())
}

#[pyfunction]
pub fn compile(program: PyProgram, chip: PyChip) -> PyResult<String> {
    let compiled_program =
        libquil_sys::compile_program(&program.into_inner().0, &chip.into_inner().0);
    Ok(format!("{compiled_program}"))
}
