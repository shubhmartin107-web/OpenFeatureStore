# Type Reference

## Enums

### `ValueType`

Feast-compatible value type enum:

```python
from ofs import ValueType

# All variants
ValueType.Invalid          # 0
ValueType.Bytes            # 1
ValueType.String           # 2
ValueType.Int32            # 3
ValueType.Int64            # 4
ValueType.Double           # 5
ValueType.Float            # 6
ValueType.Bool             # 7
ValueType.UnixTimestamp    # 8
ValueType.BytesList        # 11
ValueType.StringList       # 12
ValueType.Int32List        # 13
ValueType.Int64List        # 14
ValueType.DoubleList       # 15
ValueType.FloatList        # 16
ValueType.BoolList         # 17
ValueType.Uuid             # 36
ValueType.Decimal          # 44
ValueType.Struct           # 34
ValueType.Json             # 32

# Methods
ValueType.from_i32(2)      # Returns ValueType.String or None
vt.is_primitive()          # bool
vt.is_list()               # bool
vt.is_set()                # bool
str(vt)                    # "String"
```

### `SourceType`

```python
from ofs import SourceType

SourceType.BatchFile        # 1
SourceType.BatchBigQuery    # 2
SourceType.StreamKafka      # 3
```

### `FileFormat`

```python
from ofs import FileFormat

FileFormat.Parquet
FileFormat.Csv
FileFormat.Arrow
```

## Domain Types

### `Entity(name, join_keys)`

```python
from ofs import Entity

entity = Entity("user", ["user_id"])
entity.set_description("A user")
entity.set_owner("data-team")
entity.name          # "user"
entity.join_keys     # ["user_id"]
entity.description   # "A user"
```

### `Feature(name, value_type)`

```python
from ofs import Feature, ValueType

feature = Feature("age", ValueType.Int32)
feature.name         # "age"
feature.value_type   # ValueType.Int32
```

### `FeatureView(name)`

```python
from ofs import FeatureView, Feature

fv = FeatureView("user_features")
fv.add_entity("user")
fv.add_feature(Feature("age", ValueType.Int32))
fv.set_ttl_secs(86400)
```

### `EntityKey(join_keys)`

```python
from ofs import EntityKey

key = EntityKey(["user_id"])
key.add_value("user_id", b"12345", 2)  # ValueType.String
key.serialize()
```

### `RepoConfig()`

```python
from ofs import RepoConfig, ValueType

config = RepoConfig()
config.project = "my_project"
```
