use ofs_core::traits::{OfflineStore, OnlineStore, Registry};
use ofs_core::types::{
    DataSource, DataSourceOptions, Entity, EntityKey, Feature, FeatureView, FileFormat, RepoConfig,
    SourceType,
};
use ofs_core::value_type::ValueType;
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
use std::sync::Arc;

#[pyclass(name = "ValueType", eq, eq_int)]
#[derive(Clone, PartialEq)]
pub enum PyValueType {
    Invalid = 0,
    Bytes = 1,
    String = 2,
    Int32 = 3,
    Int64 = 4,
    Double = 5,
    Float = 6,
    Bool = 7,
    UnixTimestamp = 8,
    BytesList = 11,
    StringList = 12,
    Int32List = 13,
    Int64List = 14,
    DoubleList = 15,
    FloatList = 16,
    BoolList = 17,
    UnixTimestampList = 18,
    Null = 19,
    Map = 20,
    MapList = 21,
    BytesSet = 22,
    StringSet = 23,
    Int32Set = 24,
    Int64Set = 25,
    DoubleSet = 26,
    FloatSet = 27,
    BoolSet = 28,
    UnixTimestampSet = 29,
    Uuid = 36,
    UuidList = 38,
    UuidSet = 40,
    Decimal = 44,
    DecimalList = 45,
    DecimalSet = 46,
    Struct = 34,
    StructList = 35,
    Json = 32,
    JsonList = 33,
}

#[pymethods]
impl PyValueType {
    #[staticmethod]
    fn from_i32(v: i32) -> Option<Self> {
        ValueType::from_i32(v).map(|vt| Self::from_value_type(&vt))
    }

    fn is_primitive(&self) -> bool {
        self.to_value_type().is_primitive()
    }
    fn is_list(&self) -> bool {
        self.to_value_type().is_list()
    }
    fn is_set(&self) -> bool {
        self.to_value_type().is_set()
    }
    fn __str__(&self) -> String {
        self.to_value_type().to_string()
    }
}

impl PyValueType {
    pub fn from_value_type(vt: &ValueType) -> Self {
        match vt {
            ValueType::Invalid => Self::Invalid,
            ValueType::Bytes => Self::Bytes,
            ValueType::String => Self::String,
            ValueType::Int32 => Self::Int32,
            ValueType::Int64 => Self::Int64,
            ValueType::Double => Self::Double,
            ValueType::Float => Self::Float,
            ValueType::Bool => Self::Bool,
            ValueType::UnixTimestamp => Self::UnixTimestamp,
            ValueType::BytesList => Self::BytesList,
            ValueType::StringList => Self::StringList,
            ValueType::Int32List => Self::Int32List,
            ValueType::Int64List => Self::Int64List,
            ValueType::DoubleList => Self::DoubleList,
            ValueType::FloatList => Self::FloatList,
            ValueType::BoolList => Self::BoolList,
            ValueType::UnixTimestampList => Self::UnixTimestampList,
            ValueType::Null => Self::Null,
            ValueType::Map => Self::Map,
            ValueType::MapList => Self::MapList,
            ValueType::BytesSet => Self::BytesSet,
            ValueType::StringSet => Self::StringSet,
            ValueType::Int32Set => Self::Int32Set,
            ValueType::Int64Set => Self::Int64Set,
            ValueType::DoubleSet => Self::DoubleSet,
            ValueType::FloatSet => Self::FloatSet,
            ValueType::BoolSet => Self::BoolSet,
            ValueType::UnixTimestampSet => Self::UnixTimestampSet,
            ValueType::Uuid => Self::Uuid,
            ValueType::UuidList => Self::UuidList,
            ValueType::UuidSet => Self::UuidSet,
            ValueType::Decimal => Self::Decimal,
            ValueType::DecimalList => Self::DecimalList,
            ValueType::DecimalSet => Self::DecimalSet,
            ValueType::Struct => Self::Struct,
            ValueType::StructList => Self::StructList,
            ValueType::Json => Self::Json,
            ValueType::JsonList => Self::JsonList,
            _ => Self::Invalid,
        }
    }

    pub fn to_value_type(&self) -> ValueType {
        match self {
            Self::Invalid => ValueType::Invalid,
            Self::Bytes => ValueType::Bytes,
            Self::String => ValueType::String,
            Self::Int32 => ValueType::Int32,
            Self::Int64 => ValueType::Int64,
            Self::Double => ValueType::Double,
            Self::Float => ValueType::Float,
            Self::Bool => ValueType::Bool,
            Self::UnixTimestamp => ValueType::UnixTimestamp,
            Self::BytesList => ValueType::BytesList,
            Self::StringList => ValueType::StringList,
            Self::Int32List => ValueType::Int32List,
            Self::Int64List => ValueType::Int64List,
            Self::DoubleList => ValueType::DoubleList,
            Self::FloatList => ValueType::FloatList,
            Self::BoolList => ValueType::BoolList,
            Self::UnixTimestampList => ValueType::UnixTimestampList,
            Self::Null => ValueType::Null,
            Self::Map => ValueType::Map,
            Self::MapList => ValueType::MapList,
            Self::BytesSet => ValueType::BytesSet,
            Self::StringSet => ValueType::StringSet,
            Self::Int32Set => ValueType::Int32Set,
            Self::Int64Set => ValueType::Int64Set,
            Self::DoubleSet => ValueType::DoubleSet,
            Self::FloatSet => ValueType::FloatSet,
            Self::BoolSet => ValueType::BoolSet,
            Self::UnixTimestampSet => ValueType::UnixTimestampSet,
            Self::Uuid => ValueType::Uuid,
            Self::UuidList => ValueType::UuidList,
            Self::UuidSet => ValueType::UuidSet,
            Self::Decimal => ValueType::Decimal,
            Self::DecimalList => ValueType::DecimalList,
            Self::DecimalSet => ValueType::DecimalSet,
            Self::Struct => ValueType::Struct,
            Self::StructList => ValueType::StructList,
            Self::Json => ValueType::Json,
            Self::JsonList => ValueType::JsonList,
        }
    }
}

#[pyclass(name = "SourceType", eq, eq_int)]
#[derive(Clone, PartialEq)]
pub enum PySourceType {
    Invalid = 0,
    BatchFile = 1,
    BatchBigQuery = 2,
    StreamKafka = 3,
    StreamKinesis = 4,
    BatchRedshift = 5,
    CustomSource = 6,
    RequestSource = 7,
    BatchSnowflake = 8,
    PushSource = 9,
    BatchTrino = 10,
    BatchSpark = 11,
    BatchAthena = 12,
}

#[pymethods]
impl PySourceType {
    #[staticmethod]
    fn from_i32(v: i32) -> Option<Self> {
        SourceType::from_i32(v).map(|st| Self::from_source_type(&st))
    }
}

impl PySourceType {
    pub fn from_source_type(st: &SourceType) -> Self {
        match st {
            SourceType::Invalid => Self::Invalid,
            SourceType::BatchFile => Self::BatchFile,
            SourceType::BatchBigQuery => Self::BatchBigQuery,
            SourceType::StreamKafka => Self::StreamKafka,
            SourceType::StreamKinesis => Self::StreamKinesis,
            SourceType::BatchRedshift => Self::BatchRedshift,
            SourceType::CustomSource => Self::CustomSource,
            SourceType::RequestSource => Self::RequestSource,
            SourceType::BatchSnowflake => Self::BatchSnowflake,
            SourceType::PushSource => Self::PushSource,
            SourceType::BatchTrino => Self::BatchTrino,
            SourceType::BatchSpark => Self::BatchSpark,
            SourceType::BatchAthena => Self::BatchAthena,
        }
    }

    pub fn to_source_type(&self) -> SourceType {
        match self {
            Self::Invalid => SourceType::Invalid,
            Self::BatchFile => SourceType::BatchFile,
            Self::BatchBigQuery => SourceType::BatchBigQuery,
            Self::StreamKafka => SourceType::StreamKafka,
            Self::StreamKinesis => SourceType::StreamKinesis,
            Self::BatchRedshift => SourceType::BatchRedshift,
            Self::CustomSource => SourceType::CustomSource,
            Self::RequestSource => SourceType::RequestSource,
            Self::BatchSnowflake => SourceType::BatchSnowflake,
            Self::PushSource => SourceType::PushSource,
            Self::BatchTrino => SourceType::BatchTrino,
            Self::BatchSpark => SourceType::BatchSpark,
            Self::BatchAthena => SourceType::BatchAthena,
        }
    }
}

#[pyclass(name = "FileFormat", eq, eq_int)]
#[derive(Clone, PartialEq)]
pub enum PyFileFormat {
    Parquet = 0,
    Csv = 1,
    Arrow = 2,
}

#[pyclass(name = "Entity")]
#[derive(Clone)]
pub struct PyEntity {
    inner: Entity,
}

#[pymethods]
impl PyEntity {
    #[new]
    fn new(name: String, join_keys: Vec<String>) -> Self {
        Self {
            inner: Entity::new(&name, join_keys),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    fn join_keys(&self) -> Vec<String> {
        self.inner.join_keys.clone()
    }
    #[getter]
    fn value_type(&self) -> PyValueType {
        PyValueType::from_value_type(&self.inner.value_type)
    }
    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }
    fn set_value_type(&mut self, vt: PyValueType) {
        self.inner.value_type = vt.to_value_type();
    }
    fn set_description(&mut self, desc: String) {
        self.inner.description = desc;
    }
    fn set_owner(&mut self, owner: String) {
        self.inner.owner = owner;
    }
    fn __repr__(&self) -> String {
        format!("Entity(name={})", self.inner.name)
    }
}

impl PyEntity {
    pub fn from_entity(e: Entity) -> Self {
        Self { inner: e }
    }
    pub fn into_entity(self) -> Entity {
        self.inner
    }
    pub fn inner(&self) -> &Entity {
        &self.inner
    }
}

#[pyclass(name = "Feature")]
#[derive(Clone)]
pub struct PyFeature {
    inner: Feature,
}

#[pymethods]
impl PyFeature {
    #[new]
    fn new(name: String, value_type: PyValueType) -> Self {
        Self {
            inner: Feature::new(&name, value_type.to_value_type()),
        }
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    fn value_type(&self) -> PyValueType {
        PyValueType::from_value_type(&self.inner.value_type)
    }
    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }
    fn __repr__(&self) -> String {
        format!("Feature(name={})", self.inner.name)
    }
}

impl PyFeature {
    pub fn from_feature(f: Feature) -> Self {
        Self { inner: f }
    }
    pub fn into_feature(self) -> Feature {
        self.inner
    }
}

#[pyclass(name = "DataSource")]
#[derive(Clone)]
pub struct PyDataSource {
    inner: DataSource,
}

#[pymethods]
impl PyDataSource {
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    fn source_type(&self) -> PySourceType {
        PySourceType::from_source_type(&self.inner.source_type)
    }
    #[getter]
    fn timestamp_field(&self) -> Option<String> {
        self.inner.timestamp_field.clone()
    }
    fn __repr__(&self) -> String {
        format!("DataSource(name={})", self.inner.name)
    }
}

impl PyDataSource {
    pub fn from_data_source(ds: DataSource) -> Self {
        Self { inner: ds }
    }
    pub fn into_data_source(self) -> DataSource {
        self.inner
    }
    pub fn inner(&self) -> &DataSource {
        &self.inner
    }
}

#[pyclass(name = "DataSourceOptions")]
#[derive(Clone)]
pub struct PyDataSourceOptions {
    #[allow(dead_code)]
    inner: DataSourceOptions,
}

#[pymethods]
impl PyDataSourceOptions {
    #[staticmethod]
    fn file(path: String, file_format: &str) -> PyResult<Self> {
        let ff = match file_format {
            "parquet" => FileFormat::Parquet,
            "csv" => FileFormat::Csv,
            "arrow" => FileFormat::Arrow,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown file format: {}",
                    file_format
                )));
            }
        };
        Ok(Self {
            inner: DataSourceOptions::File {
                path,
                file_format: ff,
                s3_endpoint_override: None,
            },
        })
    }
}

#[pyclass(name = "FeatureView")]
#[derive(Clone)]
pub struct PyFeatureView {
    inner: FeatureView,
}

#[pymethods]
impl PyFeatureView {
    #[new]
    fn new(name: String) -> Self {
        Self {
            inner: FeatureView::new(&name),
        }
    }
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    #[getter]
    fn entities(&self) -> Vec<String> {
        self.inner.entities.clone()
    }
    #[getter]
    fn features(&self) -> Vec<PyFeature> {
        self.inner
            .features
            .iter()
            .map(|f| PyFeature::from_feature(f.clone()))
            .collect()
    }
    fn add_entity(&mut self, entity: String) {
        self.inner.entities.push(entity);
    }
    fn add_feature(&mut self, feature: PyFeature) {
        self.inner.features.push(feature.into_feature());
    }
    fn set_ttl_secs(&mut self, ttl_secs: i64) {
        self.inner.ttl = Some(std::time::Duration::from_secs(ttl_secs as u64));
    }
    fn __repr__(&self) -> String {
        format!("FeatureView(name={})", self.inner.name)
    }
}

impl PyFeatureView {
    pub fn from_feature_view(fv: FeatureView) -> Self {
        Self { inner: fv }
    }
    pub fn into_feature_view(self) -> FeatureView {
        self.inner
    }
    pub fn inner(&self) -> &FeatureView {
        &self.inner
    }
}

#[pyclass(name = "EntityKey")]
#[derive(Clone)]
pub struct PyEntityKey {
    inner: EntityKey,
}

#[pymethods]
impl PyEntityKey {
    #[new]
    fn new(join_keys: Vec<String>) -> Self {
        Self {
            inner: EntityKey::new(join_keys),
        }
    }

    fn add_value(&mut self, key: String, value: Vec<u8>, value_type: PyValueType) {
        self.inner.join_keys.push(key);
        self.inner.entity_values.push(value);
        self.inner.value_types.push(value_type.to_value_type());
    }

    #[getter]
    fn join_keys(&self) -> Vec<String> {
        self.inner.join_keys.clone()
    }
    fn serialize(&self) -> Vec<u8> {
        ofs_core::entity_key::serialize_entity_key_v3(&self.inner)
    }

    #[staticmethod]
    fn deserialize(bytes: Vec<u8>) -> PyResult<Self> {
        ofs_core::entity_key::deserialize_entity_key_v3(&bytes)
            .map(|ek| Self { inner: ek })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    }

    fn __repr__(&self) -> String {
        format!("EntityKey(join_keys={:?})", self.inner.join_keys)
    }
}

impl PyEntityKey {
    pub fn from_entity_key(ek: EntityKey) -> Self {
        Self { inner: ek }
    }
    pub fn into_entity_key(self) -> EntityKey {
        self.inner
    }
    pub fn inner(&self) -> &EntityKey {
        &self.inner
    }
}

#[pyclass(name = "RepoConfig")]
#[derive(Clone)]
pub struct PyRepoConfig {
    inner: RepoConfig,
}

#[pymethods]
impl PyRepoConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: RepoConfig::default(),
        }
    }
    #[getter]
    fn project(&self) -> String {
        self.inner.project.clone()
    }
    #[setter]
    fn set_project(&mut self, project: String) {
        self.inner.project = project;
    }
}

impl PyRepoConfig {
    pub fn from_config(c: RepoConfig) -> Self {
        Self { inner: c }
    }
    pub fn into_config(self) -> RepoConfig {
        self.inner
    }
    pub fn inner(&self) -> &RepoConfig {
        &self.inner
    }
}

#[pyclass(name = "FeatureService")]
#[derive(Clone)]
pub struct PyFeatureService {
    inner: ofs_core::types::FeatureService,
}

#[pymethods]
impl PyFeatureService {
    #[new]
    fn new(name: String) -> Self {
        Self {
            inner: ofs_core::types::FeatureService::new(&name),
        }
    }
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }
    fn __repr__(&self) -> String {
        format!("FeatureService(name={})", self.inner.name)
    }
}

impl PyFeatureService {
    pub fn from_service(s: ofs_core::types::FeatureService) -> Self {
        Self { inner: s }
    }
    pub fn into_service(self) -> ofs_core::types::FeatureService {
        self.inner
    }
}

pub(crate) fn extract_registry_arc(obj: &Bound<'_, PyAny>) -> PyResult<Arc<dyn Registry>> {
    let ptr: usize = obj.call_method0("_clone_registry_arc")?.extract()?;
    let b: Box<Arc<dyn Registry>> = unsafe { Box::from_raw(ptr as *mut Arc<dyn Registry>) };
    Ok(*b)
}

pub(crate) fn extract_offline_store_arc(obj: &Bound<'_, PyAny>) -> PyResult<Arc<dyn OfflineStore>> {
    let ptr: usize = obj.call_method0("_clone_offline_arc")?.extract()?;
    let b: Box<Arc<dyn OfflineStore>> = unsafe { Box::from_raw(ptr as *mut Arc<dyn OfflineStore>) };
    Ok(*b)
}

pub(crate) fn extract_online_store_arc(obj: &Bound<'_, PyAny>) -> PyResult<Arc<dyn OnlineStore>> {
    let ptr: usize = obj.call_method0("_clone_online_arc")?.extract()?;
    let b: Box<Arc<dyn OnlineStore>> = unsafe { Box::from_raw(ptr as *mut Arc<dyn OnlineStore>) };
    Ok(*b)
}
