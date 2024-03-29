use pyo3::prelude::*;
use pyo3::types::PyString;

use rigetti_pyo3::{py_wrap_struct, ToPythonError};

#[derive(Clone)]
pub struct Chip(pub(crate) libquil_sys::quilc::Chip);

py_wrap_struct! {
    PyChip(Chip) as "Chip" {
        py -> rs {
            str: Py<PyString> => Chip {
                let s = str.as_ref(py).to_str()?;
                let lc: libquil_sys::quilc::Chip = s.parse().map_err(|err| crate::RustLibquilQuilcError::from(err).to_py_err())?;
                let c: Chip = Chip(lc);
                Ok::<_, PyErr>(c)
            }
        },
    }
}
