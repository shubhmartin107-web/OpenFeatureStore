use std::sync::Arc;

use ofs_core::traits::OfflineStore;
use ofs_core::types::FeatureView;
use ofs_offline_store::DuckDbOfflineStore;
use pyo3::prelude::*;

use crate::block_on;

#[pyclass(name = "DuckDbOfflineStore")]
pub struct PyDuckDbOfflineStore {
    pub(crate) inner: Arc<dyn OfflineStore>,
}

#[pymethods]
impl PyDuckDbOfflineStore {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(DuckDbOfflineStore),
        }
    }

    fn _clone_offline_arc(&self) -> usize {
        let cloned = self.inner.clone();
        let b = Box::new(cloned);
        Box::into_raw(b) as usize
    }

    fn pull_features(
        &self,
        feature_view_name: String,
        start_date: f64,
        end_date: f64,
    ) -> PyResult<String> {
        let fv = FeatureView::new(&feature_view_name);
        let start = chrono::DateTime::from_timestamp(start_date as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid start timestamp"))?;
        let end = chrono::DateTime::from_timestamp(end_date as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid end timestamp"))?;
        let job = block_on(self.inner.pull_features(&fv, start, end))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(job.query)
    }
}
