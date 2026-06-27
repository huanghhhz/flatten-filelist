use pyo3::prelude::*;
use std::path::Path;

use crate::core;

#[pyfunction]
pub fn flatten_filelist(filelist: String, directives: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut dirs = directives;
    core::read_filelist(Path::new(&filelist), &mut dirs)
}
