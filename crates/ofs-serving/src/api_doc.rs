use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::rest::features::get_online_features_handler,
        crate::rest::features::write_online_features_handler,
        crate::rest::features::push_features_handler,
        crate::rest::health::health_handler,
        crate::rest::health::ready_handler,
        crate::rest::health::info_handler,
        crate::rest::health::metrics_handler,
    ),
    components(
        schemas(
            crate::rest::features::GetOnlineFeaturesRequestJson,
            crate::rest::features::GetOnlineFeaturesResponseJson,
            crate::rest::features::FeatureValue,
            crate::rest::features::WriteOnlineFeaturesRequestJson,
            crate::rest::features::PushFeaturesRequestJson,
            crate::rest::features::PushFeaturesResponseJson,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "features", description = "Feature serving endpoints"),
        (name = "health", description = "Health check and monitoring endpoints"),
    ),
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-api-key"))),
        );
        components.add_security_scheme(
            "bearer_token",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}
