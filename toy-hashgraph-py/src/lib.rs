use pyo3::prelude::*;

#[pyfunction]
fn add(a: i32, b: i32) -> i32 {
    ::toy_hashgraph::add(a, b)
}

#[pymodule]
#[pyo3(name = "toy_hashgraph")]
fn toy_hashgraph_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(add, m)?)?;
    Ok(())
}
