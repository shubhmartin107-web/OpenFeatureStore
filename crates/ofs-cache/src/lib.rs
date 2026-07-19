pub mod cached_store;
pub mod l1;
pub mod l2;
pub mod traits;
pub mod warming;

pub use cached_store::CachedOnlineStore;
pub use l1::L1Cache;
pub use l2::L2Cache;
pub use traits::{CacheKey, CachedValue, FeatureCache};
pub use warming::{CacheWarmer, WarmEntry};

// Re-export config types from ofs-config for convenience
pub use ofs_config::CacheConfig;
