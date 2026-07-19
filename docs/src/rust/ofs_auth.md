# ofs-auth

Authentication and authorization for OpenFeatureStore.

## AuthProvider

Trait for authenticating requests. Implementations:

- **NoopAuth** — allows all requests (default)
- **ApiKeyAuth** — validates `X-Api-Key` header against a configured list of keys with roles
- **JwtAuth** — validates Bearer tokens using JWKS from an OIDC provider, supports Keycloak `realm_access`/`resource_access` claims

## RbacChecker

Role-based access control with global roles (`admin`, `write`, `read`) and project-level roles (`admin`, `writer`, `reader`).

## Middleware

The `auth_middleware` layer intercepts HTTP requests, authenticates via the configured provider, checks RBAC against the project extracted from the path, and attaches `AuthIdentity` to request extensions. Health and metrics paths are exempt.
