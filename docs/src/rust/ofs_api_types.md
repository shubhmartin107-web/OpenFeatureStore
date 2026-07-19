# ofs-api-types

API type system for OpenFeatureStore's REST endpoints.

- **OfsApiCode** — typed API response codes mapping to HTTP status codes
- **ApiResponse<T>** — standard JSON envelope with `data`, `error`, `code`, `pagination` fields
- **Validate trait** — input validation for entity keys, feature names, project names
