use std::sync::Arc;

use ofs_core::traits::Registry;
use ofs_registry::SqlRegistry;
use pyo3::prelude::*;

use crate::block_on;
use crate::types::{PyDataSource, PyEntity, PyFeatureService, PyFeatureView};

#[pyclass(name = "SqlRegistry")]
pub struct PySqlRegistry {
    pub(crate) inner: Arc<dyn Registry>,
}

#[pymethods]
impl PySqlRegistry {
    #[staticmethod]
    fn in_memory() -> PyResult<Self> {
        let reg = block_on(SqlRegistry::in_memory())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(reg),
        })
    }

    fn _clone_registry_arc(&self) -> usize {
        let cloned = self.inner.clone();
        let b = Box::new(cloned);
        Box::into_raw(b) as usize
    }

    fn apply_entity(&self, entity: PyEntity, project: String) -> PyResult<()> {
        block_on(self.inner.apply_entity(&entity.into_entity(), &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_entity(&self, name: String, project: String) -> PyResult<Option<PyEntity>> {
        block_on(self.inner.get_entity(&name, &project))
            .map(|opt| opt.map(PyEntity::from_entity))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn list_entities(&self, project: String) -> PyResult<Vec<PyEntity>> {
        block_on(self.inner.list_entities(&project))
            .map(|v| v.into_iter().map(PyEntity::from_entity).collect())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_entity(&self, name: String, project: String) -> PyResult<()> {
        block_on(self.inner.delete_entity(&name, &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn apply_feature_view(&self, fv: PyFeatureView, project: String) -> PyResult<()> {
        block_on(
            self.inner
                .apply_feature_view(&fv.into_feature_view(), &project),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_feature_view(&self, name: String, project: String) -> PyResult<Option<PyFeatureView>> {
        block_on(self.inner.get_feature_view(&name, &project))
            .map(|opt| opt.map(PyFeatureView::from_feature_view))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn list_feature_views(&self, project: String) -> PyResult<Vec<PyFeatureView>> {
        block_on(self.inner.list_feature_views(&project))
            .map(|v| {
                v.into_iter()
                    .map(PyFeatureView::from_feature_view)
                    .collect()
            })
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_feature_view(&self, name: String, project: String) -> PyResult<()> {
        block_on(self.inner.delete_feature_view(&name, &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn apply_feature_service(&self, fs: PyFeatureService, project: String) -> PyResult<()> {
        block_on(
            self.inner
                .apply_feature_service(&fs.into_service(), &project),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_feature_service(
        &self,
        name: String,
        project: String,
    ) -> PyResult<Option<PyFeatureService>> {
        block_on(self.inner.get_feature_service(&name, &project))
            .map(|opt| opt.map(PyFeatureService::from_service))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn list_feature_services(&self, project: String) -> PyResult<Vec<PyFeatureService>> {
        block_on(self.inner.list_feature_services(&project))
            .map(|v| v.into_iter().map(PyFeatureService::from_service).collect())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_feature_service(&self, name: String, project: String) -> PyResult<()> {
        block_on(self.inner.delete_feature_service(&name, &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn apply_data_source(&self, ds: PyDataSource, project: String) -> PyResult<()> {
        block_on(
            self.inner
                .apply_data_source(&ds.into_data_source(), &project),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn get_data_source(&self, name: String, project: String) -> PyResult<Option<PyDataSource>> {
        block_on(self.inner.get_data_source(&name, &project))
            .map(|opt| opt.map(PyDataSource::from_data_source))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn list_data_sources(&self, project: String) -> PyResult<Vec<PyDataSource>> {
        block_on(self.inner.list_data_sources(&project))
            .map(|v| v.into_iter().map(PyDataSource::from_data_source).collect())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn delete_data_source(&self, name: String, project: String) -> PyResult<()> {
        block_on(self.inner.delete_data_source(&name, &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn apply_materialization(
        &self,
        fv_name: String,
        project: String,
        start: f64,
        end: f64,
    ) -> PyResult<()> {
        let start_dt = chrono::DateTime::from_timestamp(start as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid start timestamp"))?;
        let end_dt = chrono::DateTime::from_timestamp(end as i64, 0)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Invalid end timestamp"))?;
        block_on(
            self.inner
                .apply_materialization(&fv_name, &project, start_dt, end_dt),
        )
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn commit(&self) -> PyResult<()> {
        block_on(self.inner.commit())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}
