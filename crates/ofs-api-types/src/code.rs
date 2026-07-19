use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OfsApiCode {
    Ok,
    NotFound,
    AlreadyExists,
    InvalidArgument,
    Unauthenticated,
    PermissionDenied,
    Unavailable,
    Internal,
    DeadlineExceeded,
    OutOfRange,
    FailedPrecondition,
    ResourceExhausted,
}

impl OfsApiCode {
    pub fn http_status(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::NotFound => 404,
            Self::AlreadyExists => 409,
            Self::InvalidArgument => 400,
            Self::Unauthenticated => 401,
            Self::PermissionDenied => 403,
            Self::Unavailable => 503,
            Self::Internal => 500,
            Self::DeadlineExceeded => 504,
            Self::OutOfRange => 416,
            Self::FailedPrecondition => 412,
            Self::ResourceExhausted => 429,
        }
    }

    pub fn grpc_code(&self) -> i32 {
        match self {
            Self::Ok => 0,
            Self::InvalidArgument => 3,
            Self::DeadlineExceeded => 4,
            Self::NotFound => 5,
            Self::AlreadyExists => 6,
            Self::PermissionDenied => 7,
            Self::ResourceExhausted => 8,
            Self::FailedPrecondition => 9,
            Self::OutOfRange => 11,
            Self::Unauthenticated => 16,
            Self::Unavailable => 14,
            Self::Internal => 13,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::NotFound => "resource not found",
            Self::AlreadyExists => "resource already exists",
            Self::InvalidArgument => "invalid request argument",
            Self::Unauthenticated => "authentication required",
            Self::PermissionDenied => "permission denied",
            Self::Unavailable => "service temporarily unavailable",
            Self::Internal => "internal server error",
            Self::DeadlineExceeded => "deadline exceeded",
            Self::OutOfRange => "request out of range",
            Self::FailedPrecondition => "operation precondition failed",
            Self::ResourceExhausted => "resource exhausted (rate limit)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_status_mapping() {
        assert_eq!(OfsApiCode::Ok.http_status(), 200);
        assert_eq!(OfsApiCode::NotFound.http_status(), 404);
        assert_eq!(OfsApiCode::Unauthenticated.http_status(), 401);
        assert_eq!(OfsApiCode::Unavailable.http_status(), 503);
    }

    #[test]
    fn test_grpc_code_mapping() {
        assert_eq!(OfsApiCode::Ok.grpc_code(), 0);
        assert_eq!(OfsApiCode::NotFound.grpc_code(), 5);
        assert_eq!(OfsApiCode::PermissionDenied.grpc_code(), 7);
    }

    #[test]
    fn test_descriptions() {
        assert_eq!(OfsApiCode::Ok.description(), "success");
        assert_eq!(
            OfsApiCode::InvalidArgument.description(),
            "invalid request argument"
        );
    }
}
