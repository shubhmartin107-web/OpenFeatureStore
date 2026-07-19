# Value Types

OpenFeatureStore uses a Feast-compatible value type system that supports
primitives, lists, sets, and structured types.

## Primitive Types

| ValueType | Description | Variant |
|---|---|---|
| `Invalid` | Unspecified | 0 |
| `Bytes` | Raw bytes | 1 |
| `String` | UTF-8 string | 2 |
| `Int32` | 32-bit integer | 3 |
| `Int64` | 64-bit integer | 4 |
| `Double` | 64-bit float | 5 |
| `Float` | 32-bit float | 6 |
| `Bool` | Boolean | 7 |
| `UnixTimestamp` | Unix timestamp | 8 |
| `Null` | Null value | 19 |
| `Uuid` | UUID | 36 |
| `Decimal` | Decimal number | 44 |
| `Struct` | Structured data | 34 |
| `Json` | JSON value | 32 |
| `Map` | Key-value map | 20 |

## List Types

Each primitive type has a corresponding list variant:

```python
from ofs import ValueType

# String list
ValueType.StringList     # 12

# Int32 list
ValueType.Int32List      # 13

# Int64 list
ValueType.Int64List      # 14
```

## Set Types

Set types for unique value collections:

```python
ValueType.StringSet      # 23
ValueType.Int32Set       # 24
ValueType.Int64Set       # 25
```

## FeastType

The `FeastType` enum provides a richer type system that can represent
nested structures:

```rust
pub enum FeastType {
    Primitive(PrimitiveFeastType),
    Array(Box<FeastType>),
    Set(Box<FeastType>),
    Struct(Vec<(String, FeastType)>),
}
```

## Arrow Conversion

Types can be converted to Apache Arrow data types for efficient data exchange:

```rust
let arrow_type = feast_type.to_arrow_type();
```
