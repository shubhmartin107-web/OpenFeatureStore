# Entities

An **Entity** is a domain object that has features associated with it. Examples include
a user, a transaction, a product, or a geographic location.

## Definition

```rust
pub struct Entity {
    pub name: String,
    pub project: String,
    pub join_keys: Vec<String>,
    pub value_type: ValueType,
    pub description: String,
    pub tags: HashMap<String, String>,
    pub owner: String,
    pub created_timestamp: Option<DateTime<Utc>>,
    pub last_updated_timestamp: Option<DateTime<Utc>>,
}
```

## Usage

```python
from ofs import Entity

entity = Entity("user", join_keys=["user_id"])
entity.set_description("A platform user")
entity.set_owner("data-team")

store.apply_entity(entity, project="my_project")
```

## Key Points

- `join_keys` defines which columns are used to uniquely identify an entity
- An entity can have multiple join keys for composite keys
- Entities are scoped within a project
- The `value_type` field indicates the primary key's type
