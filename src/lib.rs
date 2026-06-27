pub mod core;

#[cfg(feature = "python-bindings")]
mod bindings;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

#[cfg(feature = "python-bindings")]
#[pymodule]
fn flatten_filelist(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bindings::flatten_filelist, m)?)?;
    Ok(())
}
