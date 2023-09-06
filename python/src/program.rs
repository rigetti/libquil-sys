use pyo3::prelude::*;
use pyo3::types::PyString;
use rigetti_pyo3::{py_wrap_struct, ToPythonError};

#[derive(Clone)]
pub struct Program(pub(crate) libquil_sys::Program);

py_wrap_struct! {
    PyProgram(Program) as "Program" {
        py -> rs {
            str: Py<PyString> => Chip {
                let s = str.as_ref(py).to_str()?;
                let program = s.parse().map_err(|err| crate::RustLibquilError::from(err).to_py_err())?;
                Ok::<_, PyErr>(Program(program))
            }
        },
    }
}
