//! Datastructs for python bindings.
use pyo3::{pyclass, pymethods};

#[pyclass]
#[derive(Clone)]
pub struct CombinedResultNodeSlice {
    #[pyo3(get)]
    pub begin: usize,

    #[pyo3(get)]
    pub end: usize,
}

#[pyclass]
pub struct CompiledQueryDescriptor {
    #[pyo3(get)]
    pub statement: String,

    #[pyo3(get)]
    pub combined_result_node_slices: Vec<CombinedResultNodeSlice>,
}

#[pymethods]
impl CompiledQueryDescriptor {
    #[new]
    fn new(statement: String,
           combined_result_node_slices: Vec<CombinedResultNodeSlice>)
           -> Self {
        Self { statement,
               combined_result_node_slices }
    }
}
