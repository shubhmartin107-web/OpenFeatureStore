use std::sync::Arc;

use chrono::Utc;
use ofs_core::traits::OnlineStore;
use ofs_core::types::{
    Feature, FeatureViewProjection, FeatureViewWithProjection, OnlineWriteRecord,
};
use ofs_core::value_type::ValueType;
use ofs_online_store::SqliteOnlineStore;
use pyo3::prelude::*;
use std::collections::HashMap;

use crate::block_on;
use crate::types::PyEntityKey;

#[pyclass(name = "SqliteOnlineStore")]
pub struct PySqliteOnlineStore {
    pub(crate) inner: Arc<dyn OnlineStore>,
}

#[pymethods]
impl PySqliteOnlineStore {
    #[staticmethod]
    fn in_memory() -> PyResult<Self> {
        let store = block_on(SqliteOnlineStore::in_memory())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(store),
        })
    }

    fn _clone_online_arc(&self) -> usize {
        let cloned = self.inner.clone();
        let b = Box::new(cloned);
        Box::into_raw(b) as usize
    }

    fn online_write(
        &self,
        entity_key: PyEntityKey,
        values: HashMap<String, Vec<u8>>,
        feature_view_name: String,
        project: String,
    ) -> PyResult<()> {
        let record = OnlineWriteRecord {
            entity_key: entity_key.into_entity_key(),
            values,
            timestamp: Utc::now(),
            feature_view_name,
        };
        block_on(self.inner.online_write_batch(vec![record], &project))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn online_read(
        &self,
        entity_key: PyEntityKey,
        feature_view_name: String,
        feature_names: Vec<String>,
        project: String,
    ) -> PyResult<Vec<Option<Vec<u8>>>> {
        let feature_columns: Vec<Feature> = feature_names
            .iter()
            .map(|n| Feature {
                name: n.clone(),
                value_type: ValueType::String,
                description: String::new(),
                tags: HashMap::new(),
                vector_index: false,
                vector_search_metric: None,
                vector_length: 0,
            })
            .collect();

        let fvp = FeatureViewProjection {
            feature_view_name: feature_view_name.clone(),
            feature_view_name_alias: None,
            feature_columns,
            join_key_map: HashMap::new(),
            timestamp_field: None,
            date_partition_column: None,
            created_timestamp_column: None,
            batch_source: None,
            stream_source: None,
            view_type: "regular".to_string(),
        };

        let fvwp = FeatureViewWithProjection {
            feature_view: ofs_core::types::FeatureView::new(&feature_view_name),
            projection: fvp,
        };

        let resp = block_on(self.inner.online_read(
            vec![entity_key.into_entity_key()],
            &[fvwp],
            &project,
        ))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        if resp.results.is_empty() {
            return Ok(vec![None; feature_names.len()]);
        }
        let fv = &resp.results[0];
        Ok(fv
            .values
            .iter()
            .map(|v| if v.is_empty() { None } else { Some(v.clone()) })
            .collect())
    }

    fn teardown(&self) -> PyResult<()> {
        block_on(self.inner.teardown())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}
