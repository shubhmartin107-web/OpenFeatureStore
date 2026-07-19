use async_trait::async_trait;
use ofs_config::AuthConfig;
use ofs_core::errors::{OfsError, OfsResult};
use std::collections::HashMap;

/// Identity information extracted from authentication.
#[derive(Debug, Clone)]
pub struct AuthIdentity {
    pub subject: String,
    pub roles: Vec<String>,
    pub project_roles: HashMap<String, String>,
}

/// Authentication request context.
#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub api_key: Option<String>,
    pub bearer_token: Option<String>,
}

/// Unified authentication provider trait.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn authenticate(&self, request: &AuthRequest) -> OfsResult<AuthIdentity>;
}

/// No-op authentication provider (accepts all requests).
pub struct NoopAuth;

#[async_trait]
impl AuthProvider for NoopAuth {
    async fn authenticate(&self, _request: &AuthRequest) -> OfsResult<AuthIdentity> {
        Ok(AuthIdentity {
            subject: "anonymous".to_string(),
            roles: vec!["admin".to_string()],
            project_roles: HashMap::new(),
        })
    }
}

/// API key-based authentication provider.
pub struct ApiKeyAuth {
    keys: Vec<(String, Vec<String>)>, // (key, roles)
}

impl ApiKeyAuth {
    pub fn new(config: &AuthConfig) -> Self {
        let mut keys = Vec::new();
        for entry in &config.api_keys {
            let key = entry
                .key
                .clone()
                .or_else(|| {
                    entry
                        .key_env
                        .as_ref()
                        .and_then(|env_var| std::env::var(env_var).ok())
                })
                .unwrap_or_default();
            if !key.is_empty() {
                let roles: Vec<String> = entry
                    .role
                    .as_ref()
                    .map(|r| r.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_else(|| vec!["read".to_string()]);
                keys.push((key, roles));
            }
        }
        Self { keys }
    }
}

#[async_trait]
impl AuthProvider for ApiKeyAuth {
    async fn authenticate(&self, request: &AuthRequest) -> OfsResult<AuthIdentity> {
        match &request.api_key {
            Some(key) => {
                for (stored_key, roles) in &self.keys {
                    if key == stored_key {
                        return Ok(AuthIdentity {
                            subject: format!("apikey:{}", &stored_key[..7.min(stored_key.len())]),
                            roles: roles.clone(),
                            project_roles: HashMap::new(),
                        });
                    }
                }
                Err(OfsError::Auth("Invalid API key".to_string()))
            }
            None => Err(OfsError::Auth("Missing API key".to_string())),
        }
    }
}

/// JWT/OIDC authentication provider.
///
/// Validates Bearer tokens using JWKS from an OIDC provider.
pub struct JwtAuth {
    jwks: tokio::sync::Mutex<Option<jwt::JwksCache>>,
    jwks_url: Option<String>,
    #[allow(dead_code)]
    audience: Option<String>,
    #[allow(dead_code)]
    issuer: Option<String>,
    validation: jsonwebtoken::Validation,
}

mod jwt {
    use jsonwebtoken::DecodingKey;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    #[derive(Debug, Deserialize)]
    pub struct Jwk {
        #[allow(dead_code)]
        pub kty: Option<String>,
        pub kid: Option<String>,
        pub n: Option<String>,
        pub e: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct JwksResponse {
        pub keys: Vec<Jwk>,
    }

    pub struct JwksCache {
        keys: HashMap<String, DecodingKey>,
        fetched_at: Instant,
        ttl: Duration,
        jwks_url: String,
    }

    impl JwksCache {
        pub async fn new(jwks_url: &str) -> Result<Self, String> {
            let mut cache = Self {
                keys: HashMap::new(),
                fetched_at: Instant::now(),
                ttl: Duration::from_secs(300),
                jwks_url: jwks_url.to_string(),
            };
            cache.refresh().await?;
            Ok(cache)
        }

        pub async fn get_key(&mut self, kid: &str) -> Result<DecodingKey, String> {
            if self.fetched_at.elapsed() > self.ttl {
                self.refresh().await?;
            }
            self.keys
                .get(kid)
                .cloned()
                .ok_or_else(|| format!("Key '{}' not found in JWKS", kid))
        }

        async fn refresh(&mut self) -> Result<(), String> {
            let resp: JwksResponse = reqwest::get(&self.jwks_url)
                .await
                .map_err(|e| format!("Failed to fetch JWKS: {}", e))?
                .json()
                .await
                .map_err(|e| format!("Failed to parse JWKS: {}", e))?;

            for jwk in resp.keys {
                if let (Some(n), Some(e)) = (&jwk.n, &jwk.e) {
                    let kid = jwk.kid.clone().unwrap_or_default();
                    if let Ok(key) = DecodingKey::from_rsa_components(n, e) {
                        self.keys.insert(kid, key);
                    }
                }
            }
            self.fetched_at = Instant::now();
            Ok(())
        }
    }
}

impl JwtAuth {
    pub fn new(config: &ofs_config::JwtConfig) -> Self {
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        if let Some(ref aud) = config.audience {
            validation.set_audience(&[aud.as_str()]);
        }
        if let Some(ref issuer) = config.issuer {
            validation.set_issuer(&[issuer.as_str()]);
        }
        validation.validate_exp = true;

        Self {
            jwks: tokio::sync::Mutex::new(None),
            jwks_url: config.jwks_url.clone(),
            audience: config.audience.clone(),
            issuer: config.issuer.clone(),
            validation,
        }
    }

    async fn ensure_jwks(&self) -> OfsResult<tokio::sync::MutexGuard<'_, Option<jwt::JwksCache>>> {
        let mut guard = self.jwks.lock().await;
        if guard.is_none() {
            match &self.jwks_url {
                Some(url) => {
                    let cache = jwt::JwksCache::new(url)
                        .await
                        .map_err(|e| OfsError::Auth(format!("Failed to initialize JWKS: {}", e)))?;
                    *guard = Some(cache);
                }
                None => {
                    return Err(OfsError::Auth("JWKS URL not configured".to_string()));
                }
            }
        }
        Ok(guard)
    }
}

#[async_trait]
impl AuthProvider for JwtAuth {
    async fn authenticate(&self, request: &AuthRequest) -> OfsResult<AuthIdentity> {
        let token = request
            .bearer_token
            .as_ref()
            .ok_or_else(|| OfsError::Auth("Missing Bearer token".to_string()))?;

        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| OfsError::Auth(format!("Invalid JWT header: {}", e)))?;

        let kid = header
            .kid
            .as_ref()
            .ok_or_else(|| OfsError::Auth("JWT missing 'kid' header".to_string()))?;

        let mut guard = self.ensure_jwks().await?;
        let cache = guard.as_mut().unwrap();
        let key = cache
            .get_key(kid)
            .await
            .map_err(|e| OfsError::Auth(format!("JWKS key retrieval failed: {}", e)))?;

        #[derive(Debug, serde::Deserialize)]
        struct Claims {
            sub: Option<String>,
            roles: Option<Vec<String>>,
            #[serde(default)]
            realm_access: Option<RealmAccess>,
            #[serde(default)]
            resource_access: Option<HashMap<String, ResourceRoles>>,
        }

        #[derive(Debug, serde::Deserialize)]
        struct RealmAccess {
            roles: Vec<String>,
        }

        #[derive(Debug, serde::Deserialize)]
        struct ResourceRoles {
            roles: Vec<String>,
        }

        let token_data = jsonwebtoken::decode::<Claims>(token, &key, &self.validation)
            .map_err(|e| OfsError::Auth(format!("JWT validation failed: {}", e)))?;

        let claims = token_data.claims;
        let subject = claims.sub.unwrap_or_else(|| "unknown".to_string());

        let mut roles = claims.roles.unwrap_or_default();
        if let Some(realm) = &claims.realm_access {
            roles.extend(realm.roles.clone());
        }

        let mut project_roles = HashMap::new();
        if let Some(resource) = &claims.resource_access {
            for (resource_name, resource_roles) in resource {
                for role in &resource_roles.roles {
                    project_roles.insert(resource_name.clone(), role.clone());
                }
            }
        }

        Ok(AuthIdentity {
            subject,
            roles,
            project_roles,
        })
    }
}

/// Factory to create the appropriate auth provider from config.
pub fn create_auth_provider(config: &AuthConfig) -> Box<dyn AuthProvider> {
    match config.provider.as_str() {
        "api_key" | "apikey" => {
            let provider = ApiKeyAuth::new(config);
            if provider.keys.is_empty() {
                tracing::warn!("API key auth configured but no keys found");
            }
            Box::new(provider)
        }
        "jwt" | "oidc" => match &config.jwt {
            Some(jwt_config) => Box::new(JwtAuth::new(jwt_config)),
            None => {
                tracing::warn!("JWT auth configured but no jwt config found, falling back to noop");
                Box::new(NoopAuth)
            }
        },
        _ => {
            tracing::info!("Using noop auth provider");
            Box::new(NoopAuth)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_auth() {
        let provider = NoopAuth;
        let request = AuthRequest {
            api_key: None,
            bearer_token: None,
        };
        let identity = provider.authenticate(&request).await.unwrap();
        assert_eq!(identity.subject, "anonymous");
        assert!(identity.roles.contains(&"admin".to_string()));
    }

    #[tokio::test]
    async fn test_api_key_auth_success() {
        let config = AuthConfig {
            provider: "api_key".to_string(),
            api_keys: vec![ofs_config::ApiKeyEntry {
                key: Some("sk-test-key-123".to_string()),
                key_env: None,
                role: Some("admin".to_string()),
            }],
            jwt: None,
        };
        let provider = ApiKeyAuth::new(&config);
        let request = AuthRequest {
            api_key: Some("sk-test-key-123".to_string()),
            bearer_token: None,
        };
        let identity = provider.authenticate(&request).await.unwrap();
        assert_eq!(identity.subject, "apikey:sk-test");
        assert!(identity.roles.contains(&"admin".to_string()));
    }

    #[tokio::test]
    async fn test_api_key_auth_failure() {
        let config = AuthConfig {
            provider: "api_key".to_string(),
            api_keys: vec![ofs_config::ApiKeyEntry {
                key: Some("sk-valid-key".to_string()),
                key_env: None,
                role: None,
            }],
            jwt: None,
        };
        let provider = ApiKeyAuth::new(&config);
        let request = AuthRequest {
            api_key: Some("sk-wrong-key".to_string()),
            bearer_token: None,
        };
        let result = provider.authenticate(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_key_auth_no_key_provided() {
        let config = AuthConfig {
            provider: "api_key".to_string(),
            api_keys: vec![ofs_config::ApiKeyEntry {
                key: Some("sk-valid-key".to_string()),
                key_env: None,
                role: None,
            }],
            jwt: None,
        };
        let provider = ApiKeyAuth::new(&config);
        let request = AuthRequest {
            api_key: None,
            bearer_token: None,
        };
        let result = provider.authenticate(&request).await;
        assert!(result.is_err());
    }
}
