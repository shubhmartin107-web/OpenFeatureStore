use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValueType {
    #[default]
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
    PdfBytes = 30,
    ImageBytes = 31,
    Json = 32,
    JsonList = 33,
    Struct = 34,
    StructList = 35,
    Uuid = 36,
    TimeUuid = 37,
    UuidList = 38,
    TimeUuidList = 39,
    UuidSet = 40,
    TimeUuidSet = 41,
    ValueList = 42,
    ValueSet = 43,
    Decimal = 44,
    DecimalList = 45,
    DecimalSet = 46,
    ScalarMap = 47,
    ZonedTimestamp = 48,
}

impl ValueType {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Invalid),
            1 => Some(Self::Bytes),
            2 => Some(Self::String),
            3 => Some(Self::Int32),
            4 => Some(Self::Int64),
            5 => Some(Self::Double),
            6 => Some(Self::Float),
            7 => Some(Self::Bool),
            8 => Some(Self::UnixTimestamp),
            11 => Some(Self::BytesList),
            12 => Some(Self::StringList),
            13 => Some(Self::Int32List),
            14 => Some(Self::Int64List),
            15 => Some(Self::DoubleList),
            16 => Some(Self::FloatList),
            17 => Some(Self::BoolList),
            18 => Some(Self::UnixTimestampList),
            19 => Some(Self::Null),
            20 => Some(Self::Map),
            21 => Some(Self::MapList),
            22 => Some(Self::BytesSet),
            23 => Some(Self::StringSet),
            24 => Some(Self::Int32Set),
            25 => Some(Self::Int64Set),
            26 => Some(Self::DoubleSet),
            27 => Some(Self::FloatSet),
            28 => Some(Self::BoolSet),
            29 => Some(Self::UnixTimestampSet),
            30 => Some(Self::PdfBytes),
            31 => Some(Self::ImageBytes),
            32 => Some(Self::Json),
            33 => Some(Self::JsonList),
            34 => Some(Self::Struct),
            35 => Some(Self::StructList),
            36 => Some(Self::Uuid),
            37 => Some(Self::TimeUuid),
            38 => Some(Self::UuidList),
            39 => Some(Self::TimeUuidList),
            40 => Some(Self::UuidSet),
            41 => Some(Self::TimeUuidSet),
            42 => Some(Self::ValueList),
            43 => Some(Self::ValueSet),
            44 => Some(Self::Decimal),
            45 => Some(Self::DecimalList),
            46 => Some(Self::DecimalSet),
            47 => Some(Self::ScalarMap),
            48 => Some(Self::ZonedTimestamp),
            _ => None,
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Self::Invalid
                | Self::Bytes
                | Self::String
                | Self::Int32
                | Self::Int64
                | Self::Double
                | Self::Float
                | Self::Bool
                | Self::UnixTimestamp
                | Self::Null
                | Self::PdfBytes
                | Self::ImageBytes
                | Self::Json
                | Self::Uuid
                | Self::TimeUuid
                | Self::Decimal
                | Self::ZonedTimestamp
                | Self::ScalarMap
                | Self::Map
        )
    }

    pub fn is_list(&self) -> bool {
        matches!(
            self,
            Self::BytesList
                | Self::StringList
                | Self::Int32List
                | Self::Int64List
                | Self::DoubleList
                | Self::FloatList
                | Self::BoolList
                | Self::UnixTimestampList
                | Self::JsonList
                | Self::StructList
                | Self::UuidList
                | Self::TimeUuidList
                | Self::ValueList
                | Self::DecimalList
                | Self::MapList
        )
    }

    pub fn is_set(&self) -> bool {
        matches!(
            self,
            Self::BytesSet
                | Self::StringSet
                | Self::Int32Set
                | Self::Int64Set
                | Self::DoubleSet
                | Self::FloatSet
                | Self::BoolSet
                | Self::UnixTimestampSet
                | Self::UuidSet
                | Self::TimeUuidSet
                | Self::DecimalSet
                | Self::ValueSet
        )
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Invalid => "INVALID",
            Self::Bytes => "BYTES",
            Self::String => "STRING",
            Self::Int32 => "INT32",
            Self::Int64 => "INT64",
            Self::Double => "DOUBLE",
            Self::Float => "FLOAT",
            Self::Bool => "BOOL",
            Self::UnixTimestamp => "UNIX_TIMESTAMP",
            Self::BytesList => "BYTES_LIST",
            Self::StringList => "STRING_LIST",
            Self::Int32List => "INT32_LIST",
            Self::Int64List => "INT64_LIST",
            Self::DoubleList => "DOUBLE_LIST",
            Self::FloatList => "FLOAT_LIST",
            Self::BoolList => "BOOL_LIST",
            Self::UnixTimestampList => "UNIX_TIMESTAMP_LIST",
            Self::Null => "NULL",
            Self::Map => "MAP",
            Self::MapList => "MAP_LIST",
            Self::BytesSet => "BYTES_SET",
            Self::StringSet => "STRING_SET",
            Self::Int32Set => "INT32_SET",
            Self::Int64Set => "INT64_SET",
            Self::DoubleSet => "DOUBLE_SET",
            Self::FloatSet => "FLOAT_SET",
            Self::BoolSet => "BOOL_SET",
            Self::UnixTimestampSet => "UNIX_TIMESTAMP_SET",
            Self::PdfBytes => "PDF_BYTES",
            Self::ImageBytes => "IMAGE_BYTES",
            Self::Json => "JSON",
            Self::JsonList => "JSON_LIST",
            Self::Struct => "STRUCT",
            Self::StructList => "STRUCT_LIST",
            Self::Uuid => "UUID",
            Self::TimeUuid => "TIME_UUID",
            Self::UuidList => "UUID_LIST",
            Self::TimeUuidList => "TIME_UUID_LIST",
            Self::UuidSet => "UUID_SET",
            Self::TimeUuidSet => "TIME_UUID_SET",
            Self::ValueList => "VALUE_LIST",
            Self::ValueSet => "VALUE_SET",
            Self::Decimal => "DECIMAL",
            Self::DecimalList => "DECIMAL_LIST",
            Self::DecimalSet => "DECIMAL_SET",
            Self::ScalarMap => "SCALAR_MAP",
            Self::ZonedTimestamp => "ZONED_TIMESTAMP",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ValueType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "INVALID" => Ok(Self::Invalid),
            "BYTES" => Ok(Self::Bytes),
            "STRING" => Ok(Self::String),
            "INT32" => Ok(Self::Int32),
            "INT64" => Ok(Self::Int64),
            "DOUBLE" | "FLOAT64" => Ok(Self::Double),
            "FLOAT" | "FLOAT32" => Ok(Self::Float),
            "BOOL" => Ok(Self::Bool),
            "UNIX_TIMESTAMP" => Ok(Self::UnixTimestamp),
            "BYTES_LIST" => Ok(Self::BytesList),
            "STRING_LIST" => Ok(Self::StringList),
            "INT32_LIST" => Ok(Self::Int32List),
            "INT64_LIST" => Ok(Self::Int64List),
            "DOUBLE_LIST" => Ok(Self::DoubleList),
            "FLOAT_LIST" => Ok(Self::FloatList),
            "BOOL_LIST" => Ok(Self::BoolList),
            "UNIX_TIMESTAMP_LIST" => Ok(Self::UnixTimestampList),
            "NULL" => Ok(Self::Null),
            "MAP" => Ok(Self::Map),
            "MAP_LIST" => Ok(Self::MapList),
            "BYTES_SET" => Ok(Self::BytesSet),
            "STRING_SET" => Ok(Self::StringSet),
            "INT32_SET" => Ok(Self::Int32Set),
            "INT64_SET" => Ok(Self::Int64Set),
            "DOUBLE_SET" => Ok(Self::DoubleSet),
            "FLOAT_SET" => Ok(Self::FloatSet),
            "BOOL_SET" => Ok(Self::BoolSet),
            "UNIX_TIMESTAMP_SET" => Ok(Self::UnixTimestampSet),
            "JSON" => Ok(Self::Json),
            "JSON_LIST" => Ok(Self::JsonList),
            "STRUCT" => Ok(Self::Struct),
            "STRUCT_LIST" => Ok(Self::StructList),
            "UUID" => Ok(Self::Uuid),
            "TIME_UUID" => Ok(Self::TimeUuid),
            "UUID_LIST" => Ok(Self::UuidList),
            "TIME_UUID_LIST" => Ok(Self::TimeUuidList),
            "UUID_SET" => Ok(Self::UuidSet),
            "TIME_UUID_SET" => Ok(Self::TimeUuidSet),
            "VALUE_LIST" => Ok(Self::ValueList),
            "VALUE_SET" => Ok(Self::ValueSet),
            "DECIMAL" => Ok(Self::Decimal),
            "DECIMAL_LIST" => Ok(Self::DecimalList),
            "DECIMAL_SET" => Ok(Self::DecimalSet),
            "SCALAR_MAP" => Ok(Self::ScalarMap),
            "ZONED_TIMESTAMP" => Ok(Self::ZonedTimestamp),
            _ => Err(format!("Unknown ValueType: {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveFeastType {
    Invalid,
    String,
    Bytes,
    Bool,
    Int32,
    Int64,
    Float32,
    Float64,
    UnixTimestamp,
    Json,
    Uuid,
    TimeUuid,
    Decimal,
    ScalarMap,
    ZonedTimestamp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeastType {
    Primitive(PrimitiveFeastType),
    Array(Box<FeastType>),
    Set(Box<FeastType>),
    Struct(Vec<(String, FeastType)>),
}

impl FeastType {
    pub fn from_value_type(vt: ValueType) -> Self {
        match vt {
            ValueType::Invalid => Self::Primitive(PrimitiveFeastType::Invalid),
            ValueType::String => Self::Primitive(PrimitiveFeastType::String),
            ValueType::Bytes => Self::Primitive(PrimitiveFeastType::Bytes),
            ValueType::Bool => Self::Primitive(PrimitiveFeastType::Bool),
            ValueType::Int32 => Self::Primitive(PrimitiveFeastType::Int32),
            ValueType::Int64 => Self::Primitive(PrimitiveFeastType::Int64),
            ValueType::Float => Self::Primitive(PrimitiveFeastType::Float32),
            ValueType::Double => Self::Primitive(PrimitiveFeastType::Float64),
            ValueType::UnixTimestamp => Self::Primitive(PrimitiveFeastType::UnixTimestamp),
            ValueType::Json => Self::Primitive(PrimitiveFeastType::Json),
            ValueType::Uuid => Self::Primitive(PrimitiveFeastType::Uuid),
            ValueType::TimeUuid => Self::Primitive(PrimitiveFeastType::TimeUuid),
            ValueType::Decimal => Self::Primitive(PrimitiveFeastType::Decimal),
            ValueType::ScalarMap => Self::Primitive(PrimitiveFeastType::ScalarMap),
            ValueType::ZonedTimestamp => Self::Primitive(PrimitiveFeastType::ZonedTimestamp),
            ValueType::Null => Self::Primitive(PrimitiveFeastType::Invalid),
            ValueType::BytesList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Bytes)))
            }
            ValueType::StringList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::String)))
            }
            ValueType::Int32List => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Int32)))
            }
            ValueType::Int64List => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Int64)))
            }
            ValueType::DoubleList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Float64)))
            }
            ValueType::FloatList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Float32)))
            }
            ValueType::BoolList => Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Bool))),
            ValueType::UnixTimestampList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::UnixTimestamp)))
            }
            ValueType::JsonList => Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Json))),
            ValueType::UuidList => Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Uuid))),
            ValueType::TimeUuidList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::TimeUuid)))
            }
            ValueType::DecimalList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Decimal)))
            }
            ValueType::Map => Self::Struct(vec![]),
            ValueType::MapList => Self::Array(Box::new(Self::Struct(vec![]))),
            ValueType::Struct => Self::Struct(vec![]),
            ValueType::StructList => Self::Array(Box::new(Self::Struct(vec![]))),
            ValueType::BytesSet => Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Bytes))),
            ValueType::StringSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::String)))
            }
            ValueType::Int32Set => Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Int32))),
            ValueType::Int64Set => Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Int64))),
            ValueType::DoubleSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Float64)))
            }
            ValueType::FloatSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Float32)))
            }
            ValueType::BoolSet => Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Bool))),
            ValueType::UnixTimestampSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::UnixTimestamp)))
            }
            ValueType::UuidSet => Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Uuid))),
            ValueType::TimeUuidSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::TimeUuid)))
            }
            ValueType::DecimalSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Decimal)))
            }
            ValueType::ValueList => {
                Self::Array(Box::new(Self::Primitive(PrimitiveFeastType::Invalid)))
            }
            ValueType::ValueSet => {
                Self::Set(Box::new(Self::Primitive(PrimitiveFeastType::Invalid)))
            }
            ValueType::PdfBytes | ValueType::ImageBytes => {
                Self::Primitive(PrimitiveFeastType::Bytes)
            }
        }
    }

    pub fn to_value_type(&self) -> ValueType {
        match self {
            Self::Primitive(p) => match p {
                PrimitiveFeastType::Invalid => ValueType::Invalid,
                PrimitiveFeastType::String => ValueType::String,
                PrimitiveFeastType::Bytes => ValueType::Bytes,
                PrimitiveFeastType::Bool => ValueType::Bool,
                PrimitiveFeastType::Int32 => ValueType::Int32,
                PrimitiveFeastType::Int64 => ValueType::Int64,
                PrimitiveFeastType::Float32 => ValueType::Float,
                PrimitiveFeastType::Float64 => ValueType::Double,
                PrimitiveFeastType::UnixTimestamp => ValueType::UnixTimestamp,
                PrimitiveFeastType::Json => ValueType::Json,
                PrimitiveFeastType::Uuid => ValueType::Uuid,
                PrimitiveFeastType::TimeUuid => ValueType::TimeUuid,
                PrimitiveFeastType::Decimal => ValueType::Decimal,
                PrimitiveFeastType::ScalarMap => ValueType::ScalarMap,
                PrimitiveFeastType::ZonedTimestamp => ValueType::ZonedTimestamp,
            },
            Self::Array(inner) => match inner.as_ref() {
                Self::Primitive(p) => match p {
                    PrimitiveFeastType::Bytes => ValueType::BytesList,
                    PrimitiveFeastType::String => ValueType::StringList,
                    PrimitiveFeastType::Int32 => ValueType::Int32List,
                    PrimitiveFeastType::Int64 => ValueType::Int64List,
                    PrimitiveFeastType::Float32 => ValueType::FloatList,
                    PrimitiveFeastType::Float64 => ValueType::DoubleList,
                    PrimitiveFeastType::Bool => ValueType::BoolList,
                    PrimitiveFeastType::UnixTimestamp => ValueType::UnixTimestampList,
                    PrimitiveFeastType::Json => ValueType::JsonList,
                    PrimitiveFeastType::Uuid => ValueType::UuidList,
                    PrimitiveFeastType::TimeUuid => ValueType::TimeUuidList,
                    PrimitiveFeastType::Decimal => ValueType::DecimalList,
                    _ => ValueType::ValueList,
                },
                Self::Struct(_) => ValueType::StructList,
                _ => ValueType::ValueList,
            },
            Self::Set(inner) => match inner.as_ref() {
                Self::Primitive(p) => match p {
                    PrimitiveFeastType::Bytes => ValueType::BytesSet,
                    PrimitiveFeastType::String => ValueType::StringSet,
                    PrimitiveFeastType::Int32 => ValueType::Int32Set,
                    PrimitiveFeastType::Int64 => ValueType::Int64Set,
                    PrimitiveFeastType::Float32 => ValueType::FloatSet,
                    PrimitiveFeastType::Float64 => ValueType::DoubleSet,
                    PrimitiveFeastType::Bool => ValueType::BoolSet,
                    PrimitiveFeastType::UnixTimestamp => ValueType::UnixTimestampSet,
                    PrimitiveFeastType::Uuid => ValueType::UuidSet,
                    PrimitiveFeastType::TimeUuid => ValueType::TimeUuidSet,
                    PrimitiveFeastType::Decimal => ValueType::DecimalSet,
                    _ => ValueType::ValueSet,
                },
                _ => ValueType::ValueSet,
            },
            Self::Struct(_) => ValueType::Struct,
        }
    }

    pub fn to_arrow_type(&self) -> arrow::datatypes::DataType {
        match self {
            Self::Primitive(p) => match p {
                PrimitiveFeastType::Invalid => arrow::datatypes::DataType::Null,
                PrimitiveFeastType::String => arrow::datatypes::DataType::Utf8,
                PrimitiveFeastType::Bytes => arrow::datatypes::DataType::Binary,
                PrimitiveFeastType::Bool => arrow::datatypes::DataType::Boolean,
                PrimitiveFeastType::Int32 => arrow::datatypes::DataType::Int32,
                PrimitiveFeastType::Int64 | PrimitiveFeastType::UnixTimestamp => {
                    arrow::datatypes::DataType::Int64
                }
                PrimitiveFeastType::Float32 => arrow::datatypes::DataType::Float32,
                PrimitiveFeastType::Float64 => arrow::datatypes::DataType::Float64,
                PrimitiveFeastType::Json
                | PrimitiveFeastType::Uuid
                | PrimitiveFeastType::TimeUuid
                | PrimitiveFeastType::Decimal => arrow::datatypes::DataType::Utf8,
                PrimitiveFeastType::ScalarMap => arrow::datatypes::DataType::Map(
                    std::sync::Arc::new(arrow::datatypes::Field::new(
                        "entries",
                        arrow::datatypes::DataType::Struct(
                            vec![
                                arrow::datatypes::Field::new(
                                    "key",
                                    arrow::datatypes::DataType::Utf8,
                                    false,
                                ),
                                arrow::datatypes::Field::new(
                                    "value",
                                    arrow::datatypes::DataType::Utf8,
                                    true,
                                ),
                            ]
                            .into(),
                        ),
                        false,
                    )),
                    false,
                ),
                PrimitiveFeastType::ZonedTimestamp => arrow::datatypes::DataType::Timestamp(
                    arrow::datatypes::TimeUnit::Microsecond,
                    Some("UTC".into()),
                ),
            },
            Self::Array(inner) => {
                let value_field = arrow::datatypes::Field::new("item", inner.to_arrow_type(), true);
                arrow::datatypes::DataType::List(std::sync::Arc::new(value_field))
            }
            Self::Set(inner) => {
                let value_field = arrow::datatypes::Field::new("item", inner.to_arrow_type(), true);
                arrow::datatypes::DataType::List(std::sync::Arc::new(value_field))
            }
            Self::Struct(fields) => {
                let arrow_fields: Vec<arrow::datatypes::Field> = fields
                    .iter()
                    .map(|(name, ft)| {
                        arrow::datatypes::Field::new(name.as_str(), ft.to_arrow_type(), true)
                    })
                    .collect();
                arrow::datatypes::DataType::Struct(arrow_fields.into())
            }
        }
    }
}

impl fmt::Display for FeastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(p) => write!(f, "{p}"),
            Self::Array(inner) => write!(f, "Array<{inner}>"),
            Self::Set(inner) => write!(f, "Set<{inner}>"),
            Self::Struct(fields) => {
                write!(
                    f,
                    "Struct<{}>",
                    fields
                        .iter()
                        .map(|(n, t)| format!("{n}: {t}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl fmt::Display for PrimitiveFeastType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Invalid => "Invalid",
            Self::String => "String",
            Self::Bytes => "Bytes",
            Self::Bool => "Bool",
            Self::Int32 => "Int32",
            Self::Int64 => "Int64",
            Self::Float32 => "Float32",
            Self::Float64 => "Float64",
            Self::UnixTimestamp => "UnixTimestamp",
            Self::Json => "Json",
            Self::Uuid => "Uuid",
            Self::TimeUuid => "TimeUuid",
            Self::Decimal => "Decimal",
            Self::ScalarMap => "ScalarMap",
            Self::ZonedTimestamp => "ZonedTimestamp",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_from_i32() {
        assert_eq!(ValueType::from_i32(0), Some(ValueType::Invalid));
        assert_eq!(ValueType::from_i32(2), Some(ValueType::String));
        assert_eq!(ValueType::from_i32(4), Some(ValueType::Int64));
        assert_eq!(ValueType::from_i32(48), Some(ValueType::ZonedTimestamp));
        assert_eq!(ValueType::from_i32(999), None);
    }

    #[test]
    fn test_value_type_display() {
        assert_eq!(ValueType::String.to_string(), "STRING");
        assert_eq!(ValueType::Int64.to_string(), "INT64");
        assert_eq!(ValueType::ZonedTimestamp.to_string(), "ZONED_TIMESTAMP");
    }

    #[test]
    fn test_value_type_from_str() {
        assert_eq!("STRING".parse::<ValueType>().unwrap(), ValueType::String);
        assert_eq!("int64".parse::<ValueType>().unwrap(), ValueType::Int64);
        assert_eq!("FLOAT64".parse::<ValueType>().unwrap(), ValueType::Double);
        assert!("UNKNOWN".parse::<ValueType>().is_err());
    }

    #[test]
    fn test_primitive_check() {
        assert!(ValueType::String.is_primitive());
        assert!(ValueType::Int64.is_primitive());
        assert!(!ValueType::StringList.is_primitive());
        assert!(!ValueType::Int32Set.is_primitive());
    }

    #[test]
    fn test_list_check() {
        assert!(ValueType::StringList.is_list());
        assert!(ValueType::Int64List.is_list());
        assert!(!ValueType::String.is_list());
    }

    #[test]
    fn test_set_check() {
        assert!(ValueType::StringSet.is_set());
        assert!(ValueType::Int64Set.is_set());
        assert!(!ValueType::String.is_set());
    }

    #[test]
    fn test_feast_type_round_trip() {
        let cases = vec![
            ValueType::String,
            ValueType::Int64,
            ValueType::Double,
            ValueType::Bool,
            ValueType::StringList,
            ValueType::Int64List,
            ValueType::StringSet,
            ValueType::UnixTimestamp,
            ValueType::Json,
            ValueType::Uuid,
            ValueType::Decimal,
        ];
        for vt in cases {
            let ft = FeastType::from_value_type(vt);
            assert_eq!(ft.to_value_type(), vt, "Round-trip failed for {vt}");
        }
    }

    #[test]
    fn test_feast_type_to_arrow() {
        let ft = FeastType::Primitive(PrimitiveFeastType::Int64);
        assert_eq!(ft.to_arrow_type(), arrow::datatypes::DataType::Int64);

        let ft = FeastType::Primitive(PrimitiveFeastType::String);
        assert_eq!(ft.to_arrow_type(), arrow::datatypes::DataType::Utf8);

        let ft = FeastType::Primitive(PrimitiveFeastType::Float32);
        assert_eq!(ft.to_arrow_type(), arrow::datatypes::DataType::Float32);

        let ft = FeastType::Array(Box::new(FeastType::Primitive(PrimitiveFeastType::Int64)));
        assert_eq!(
            ft.to_arrow_type(),
            arrow::datatypes::DataType::List(std::sync::Arc::new(arrow::datatypes::Field::new(
                "item",
                arrow::datatypes::DataType::Int64,
                true
            )))
        );
    }
}
