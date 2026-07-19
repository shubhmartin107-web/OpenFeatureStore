mod materialization;
mod offline_store;
mod online_store;
mod registry;
mod types;

use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::sync::Mutex;
use tokio::runtime::Runtime;

static RUNTIME: Lazy<Mutex<Runtime>> =
    Lazy::new(|| Mutex::new(Runtime::new().expect("Failed to create Tokio runtime")));

pub(crate) fn block_on<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    RUNTIME.lock().unwrap().block_on(f)
}

#[pymodule]
fn _rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<types::PyValueType>()?;
    m.add_class::<types::PySourceType>()?;
    m.add_class::<types::PyFileFormat>()?;
    m.add_class::<types::PyEntity>()?;
    m.add_class::<types::PyFeature>()?;
    m.add_class::<types::PyFeatureView>()?;
    m.add_class::<types::PyDataSource>()?;
    m.add_class::<types::PyDataSourceOptions>()?;
    m.add_class::<types::PyEntityKey>()?;
    m.add_class::<types::PyRepoConfig>()?;
    m.add_class::<types::PyFeatureService>()?;
    m.add_class::<registry::PySqlRegistry>()?;
    m.add_class::<offline_store::PyDuckDbOfflineStore>()?;
    m.add_class::<online_store::PySqliteOnlineStore>()?;
    m.add_class::<materialization::PyMaterializationEngine>()?;
    Ok(())
}
