# Entity Keys

An **EntityKey** uniquely identifies an entity instance. It supports composite keys
(multiple join key values) and serializes to a byte representation for storage.

## Definition

```rust
pub struct EntityKey {
    pub join_keys: Vec<String>,
    pub entity_values: Vec<Vec<u8>>,
    pub value_types: Vec<ValueType>,
}
```

## Serialization

Feast v3 entity key serialization is used for storage in the online store:

```python
from ofs import EntityKey

key = EntityKey(["user_id"])
key.add_value("user_id", b"12345", 2)  # ValueType.String

serialized = key.serialize()
# Returns Feast v3 format bytes

# Deserialize
restored = EntityKey.deserialize(serialized)
```

## Key Format

The Feast v3 entity key format serializes multiple key-value pairs using a
compact binary encoding:

- Prefix identifies the serialization version
- Each key-value pair is length-prefixed
- Values are encoded according to their value type

## Prefix Scanning

The `serialize_entity_key_prefix` function enables prefix-based scanning
for partial key matches:

```rust
let prefix = serialize_entity_key_prefix(&["user_id".to_string()]);
```
