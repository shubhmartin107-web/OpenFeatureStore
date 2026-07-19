# ofs-proto

Generated protobuf definitions matching Feast 0.42+ schema.

## Protobuf Files

The protobuf definitions are in `protos/feast/`:

- `core/DataSource.proto` — Data source definitions
- `core/Entity.proto` — Entity definitions
- `core/FeatureService.proto` — Feature service definitions
- `core/FeatureTable.proto` — Feature table (legacy)
- `core/FeatureView.proto` — Feature view definitions
- `core/FeatureViewProjection.proto` — Feature view projection
- `core/OnDemandFeatureView.proto` — On-demand feature views
- `core/Registry.proto` — Registry serialization
- `core/SqlRegistry.proto` — SQL registry metadata
- `core/DataFormat.proto` — Data format definitions
- `core/Aggregation.proto` — Aggregation definitions
- `types/Value.proto` — Value type definitions
- `serving/ServingService.proto` — Serving service API

## Code Generation

Protobuf code is generated at build time using `prost-build`:

```rust,no_run
// build.rs
fn main() {
    prost_build::compile_protos(
        &["protos/feast/core/Registry.proto"],
        &["protos/"],
    ).unwrap();
}
```

## Usage

```rust
use ofs_proto::feast::core::Entity as ProtoEntity;

let entity = ProtoEntity {
    name: "user".to_string(),
    // ...
};
```
