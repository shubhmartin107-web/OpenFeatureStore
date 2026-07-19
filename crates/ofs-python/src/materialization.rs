use ofs_core::traits::MaterializationEngine;
use ofs_core::types::RepoConfig;
use ofs_materialization::DefaultMaterializationEngine;
use pyo3::prelude::*;

use crate::block_on;
use crate::types::{extract_offline_store_arc, extract_online_store_arc, extract_registry_arc};

#[pyclass(name = "DefaultMaterializationEngine")]
pub struct PyMaterializationEngine {
    inner: DefaultMaterializationEngine,
}

#[pymethods]
impl PyMaterializationEngine {
    #[staticmethod]
    fn create(
        registry: &Bound<'_, PyAny>,
        offline_store: &Bound<'_, PyAny>,
        online_store: &Bound<'_, PyAny>,
        project: Option<String>,
    ) -> PyResult<Self> {
        let reg = extract_registry_arc(registry)?;
        let off = extract_offline_store_arc(offline_store)?;
        let on = extract_online_store_arc(online_store)?;
        let config = RepoConfig {
            project: project.unwrap_or_else(|| "default".to_string()),
            ..RepoConfig::default()
        };
        Ok(Self {
            inner: DefaultMaterializationEngine::new(reg, off, on, config),
        })
    }

    fn materialize(&self, start_date: f64, end_date: f64, project: String) -> PyResult<()> {
        let start = chrono::DateTime::from_timestamp(start_date as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid start timestamp"))?;
        let end = chrono::DateTime::from_timestamp(end_date as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid end timestamp"))?;
        block_on(self.inner.materialize(start, end, None, &project, false))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn materialize_incremental(&self, end_date: f64, project: String) -> PyResult<()> {
        let end = chrono::DateTime::from_timestamp(end_date as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid end timestamp"))?;
        block_on(
            self.inner
                .materialize_incremental(end, None, &project, false),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}
