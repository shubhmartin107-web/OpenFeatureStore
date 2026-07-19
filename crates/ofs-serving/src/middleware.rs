use std::sync::Arc;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};

#[derive(Clone)]
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    pub fn new(rps: u32) -> Self {
        let quota =
            Quota::per_second(std::num::NonZeroU32::new(rps.max(1)).expect("rps must be non-zero"));
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));
        Self { limiter }
    }

    pub async fn check(&self, req: Request, next: Next) -> Response {
        let is_health = req.uri().path().starts_with("/v1/health")
            || req.uri().path().starts_with("/v1/ready")
            || req.uri().path().starts_with("/v1/metrics")
            || req.uri().path().starts_with("/docs");

        if is_health {
            return next.run(req).await;
        }

        match self.limiter.check() {
            Ok(()) => next.run(req).await,
            Err(_) => {
                let resp = (
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    [("Retry-After", "1")],
                    "rate limit exceeded",
                );
                resp.into_response()
            }
        }
    }
}

pub fn make_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("req-{ts:x}-{id:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_format() {
        let id = make_request_id();
        assert!(id.starts_with("req-"));
        assert!(id.len() > 10);
    }

    #[test]
    fn test_request_id_unique() {
        let a = make_request_id();
        let b = make_request_id();
        assert_ne!(a, b);
    }
}
