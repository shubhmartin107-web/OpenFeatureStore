use crate::types::EntityKey;
use crate::value_type::ValueType;

/// Serialize an entity key using v3 format.
///
/// Byte layout:
/// - [4 bytes] num_keys: u32 LE
/// - For each key in sorted(keys):
///   - [4 bytes] key_type: u32 LE = ValueType::STRING (2)
///   - [4 bytes] key_len: u32 LE
///   - [key_len bytes] key_name: UTF-8
/// - For each value in same order as sorted keys:
///   - [4 bytes] val_type: u32 LE
///   - [4 bytes] val_len: u32 LE
///   - [val_len bytes] val_bytes
pub fn serialize_entity_key_v3(entity_key: &EntityKey) -> Vec<u8> {
    let mut buf = Vec::new();

    // Sort join_keys and entity_values together alphabetically by key name
    let mut pairs: Vec<(String, &[u8], ValueType)> = entity_key
        .join_keys
        .iter()
        .zip(entity_key.value_types.iter())
        .zip(entity_key.entity_values.iter())
        .map(|((k, vt), v)| (k.clone(), v.as_slice(), *vt))
        .collect();

    if pairs.len() > 1 {
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let num_keys = pairs.len() as u32;
    buf.extend_from_slice(&num_keys.to_le_bytes());

    for (key_name, _, _) in &pairs {
        let key_bytes = key_name.as_bytes();
        let key_type: u32 = ValueType::String as u32;
        buf.extend_from_slice(&key_type.to_le_bytes());
        buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(key_bytes);
    }

    for (_, val_bytes, val_type) in &pairs {
        let vt_val: u32 = *val_type as u32;
        buf.extend_from_slice(&vt_val.to_le_bytes());
        buf.extend_from_slice(&(val_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(val_bytes);
    }

    buf
}

/// Deserialize an entity key from v3 format.
pub fn deserialize_entity_key_v3(bytes: &[u8]) -> Result<EntityKey, String> {
    let mut offset = 0;
    if bytes.len() < 4 {
        return Err("Entity key too short".to_string());
    }

    let num_keys = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;

    let mut join_keys = Vec::with_capacity(num_keys);

    for _ in 0..num_keys {
        if offset + 8 > bytes.len() {
            return Err("Unexpected end of key section".to_string());
        }
        let _key_type = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let key_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + key_len > bytes.len() {
            return Err("Key name truncated".to_string());
        }
        let key_name = String::from_utf8(bytes[offset..offset + key_len].to_vec())
            .map_err(|e| e.to_string())?;
        offset += key_len;
        join_keys.push(key_name);
    }

    let mut entity_values = Vec::with_capacity(num_keys);
    let mut value_types = Vec::with_capacity(num_keys);

    for _ in 0..num_keys {
        if offset + 8 > bytes.len() {
            return Err("Unexpected end of value section".to_string());
        }
        let vt_raw = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let val_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + val_len > bytes.len() {
            return Err("Value truncated".to_string());
        }
        let val_bytes = bytes[offset..offset + val_len].to_vec();
        offset += val_len;
        entity_values.push(val_bytes);

        let value_type = ValueType::from_i32(vt_raw as i32).unwrap_or(ValueType::Invalid);
        value_types.push(value_type);
    }

    Ok(EntityKey {
        join_keys,
        entity_values,
        value_types,
    })
}

/// Serialize only the key names (no values) for prefix scanning.
pub fn serialize_entity_key_prefix(join_keys: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    let num_keys = join_keys.len() as u32;
    buf.extend_from_slice(&num_keys.to_le_bytes());

    let mut sorted_keys: Vec<&String> = join_keys.iter().collect();
    if sorted_keys.len() > 1 {
        sorted_keys.sort();
    }

    for key_name in &sorted_keys {
        let key_bytes = key_name.as_bytes();
        let key_type: u32 = ValueType::String as u32;
        buf.extend_from_slice(&key_type.to_le_bytes());
        buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(key_bytes);
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity_key(keys: Vec<&str>, values: Vec<(ValueType, &[u8])>) -> EntityKey {
        EntityKey {
            join_keys: keys.iter().map(|s| s.to_string()).collect(),
            entity_values: values.iter().map(|(_, v)| v.to_vec()).collect(),
            value_types: values.iter().map(|(vt, _)| *vt).collect(),
        }
    }

    #[test]
    fn test_serialize_deserialize_single_key() {
        let ek = make_entity_key(
            vec!["driver_id"],
            vec![(ValueType::Int64, &1002i64.to_le_bytes())],
        );
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        assert_eq!(ek.join_keys, deserialized.join_keys);
        assert_eq!(ek.entity_values, deserialized.entity_values);
        assert_eq!(ek.value_types, deserialized.value_types);
    }

    #[test]
    fn test_serialize_deserialize_multi_key() {
        let ek = make_entity_key(
            vec!["customer_id", "region"],
            vec![
                (ValueType::Int64, &123i64.to_le_bytes()),
                (ValueType::String, b"us-east"),
            ],
        );
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        // Keys should be sorted alphabetically
        assert_eq!(deserialized.join_keys[0], "customer_id");
        assert_eq!(deserialized.join_keys[1], "region");
        // Values should match in sorted order
        assert_eq!(deserialized.value_types[0], ValueType::Int64);
        assert_eq!(deserialized.value_types[1], ValueType::String);
    }

    #[test]
    fn test_serialize_deserialize_string_key() {
        let ek = make_entity_key(vec!["user_id"], vec![(ValueType::String, b"user_abc123")]);
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        assert_eq!(deserialized.join_keys[0], "user_id");
        assert_eq!(deserialized.entity_values[0], b"user_abc123");
        assert_eq!(deserialized.value_types[0], ValueType::String);
    }

    #[test]
    fn test_serialize_deserialize_int32_key() {
        let ek = make_entity_key(
            vec!["score"],
            vec![(ValueType::Int32, &42i32.to_le_bytes())],
        );
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        assert_eq!(deserialized.value_types[0], ValueType::Int32);
        assert_eq!(deserialized.entity_values[0], 42i32.to_le_bytes().to_vec());
    }

    #[test]
    fn test_serialize_deserialize_double_key() {
        let ek = make_entity_key(
            vec!["amount"],
            vec![(ValueType::Double, &3.14f64.to_le_bytes())],
        );
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        assert_eq!(deserialized.value_types[0], ValueType::Double);
        assert_eq!(
            deserialized.entity_values[0],
            3.14f64.to_le_bytes().to_vec()
        );
    }

    #[test]
    fn test_serialize_entity_key_prefix() {
        let prefix = serialize_entity_key_prefix(&["driver_id".to_string()]);
        assert!(prefix.len() >= 4);
        // Prefix starts with num_keys (4 bytes) + key_type (4 bytes) + key_len (4 bytes) + key_name
        let num_keys = u32::from_le_bytes(prefix[..4].try_into().unwrap());
        assert_eq!(num_keys, 1);

        let multi_prefix =
            serialize_entity_key_prefix(&["customer_id".to_string(), "region".to_string()]);
        // Multiple keys should produce a longer prefix
        assert!(multi_prefix.len() > prefix.len());
        let multi_num_keys = u32::from_le_bytes(multi_prefix[..4].try_into().unwrap());
        assert_eq!(multi_num_keys, 2);

        // Keys in prefix should be sorted alphabetically
        // After the header: first key should be "customer_id" (alphabetically before "region")
        let key1_len_offset = 4 + 4; // skip num_keys + key_type
        let key1_len = u32::from_le_bytes(
            multi_prefix[key1_len_offset..key1_len_offset + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let key1_start = key1_len_offset + 4;
        let key1 =
            String::from_utf8(multi_prefix[key1_start..key1_start + key1_len].to_vec()).unwrap();
        assert_eq!(key1, "customer_id");
    }

    #[test]
    fn test_deserialize_invalid_short() {
        assert!(deserialize_entity_key_v3(&[0x01, 0x00]).is_err());
    }

    #[test]
    fn test_entity_key_size() {
        let ek = make_entity_key(
            vec!["driver_id"],
            vec![(ValueType::Int64, &1002i64.to_le_bytes())],
        );
        let serialized = serialize_entity_key_v3(&ek);
        // 4 (num_keys) + 4 (key_type) + 4 (key_len) + 9 (key) + 4 (val_type) + 4 (val_len) + 8 (val)
        assert_eq!(serialized.len(), 37);
    }

    #[test]
    fn test_multiple_keys_in_order() {
        // Test that keys are sorted alphabetically, not insertion order
        let ek = make_entity_key(
            vec!["z_key", "a_key"],
            vec![(ValueType::String, b"z_val"), (ValueType::String, b"a_val")],
        );
        let serialized = serialize_entity_key_v3(&ek);
        let deserialized = deserialize_entity_key_v3(&serialized).unwrap();
        assert_eq!(deserialized.join_keys[0], "a_key");
        assert_eq!(deserialized.join_keys[1], "z_key");
        assert_eq!(deserialized.entity_values[0], b"a_val");
        assert_eq!(deserialized.entity_values[1], b"z_val");
    }
}
