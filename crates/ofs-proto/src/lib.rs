#![allow(clippy::large_enum_variant)]

pub mod feast {
    pub mod types {
        include!(concat!(env!("OUT_DIR"), "/feast.types.rs"));
    }
    pub mod core {
        include!(concat!(env!("OUT_DIR"), "/feast.core.rs"));
    }
    pub mod serving {
        include!(concat!(env!("OUT_DIR"), "/feast.serving.rs"));
    }
}

pub use feast::core as core_proto;
pub use feast::serving as serving_proto;
pub use feast::types as types_proto;
