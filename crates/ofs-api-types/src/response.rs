use serde::{Deserialize, Serialize};

use crate::code::OfsApiCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub field: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub status: String,
    pub code: OfsApiCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ApiErrorDetail>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            status: "success".into(),
            code: OfsApiCode::Ok,
            message: String::new(),
            data: Some(data),
            errors: None,
            request_id: None,
        }
    }

    pub fn error(code: OfsApiCode, message: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            code,
            message: message.into(),
            data: None,
            errors: None,
            request_id: None,
        }
    }

    pub fn with_errors(mut self, errors: Vec<ApiErrorDetail>) -> Self {
        self.errors = Some(errors);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn into_json(self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({
            "status": "error",
            "code": "INTERNAL",
            "message": "failed to serialize response"
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub has_more: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let has_more = (page * page_size) < total;
        Self {
            data,
            total,
            page,
            page_size,
            has_more,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let resp: ApiResponse<String> = ApiResponse::success("hello".into());
        assert_eq!(resp.status, "success");
        assert_eq!(resp.code, OfsApiCode::Ok);
        assert_eq!(resp.data, Some("hello".into()));
        assert!(resp.errors.is_none());
    }

    #[test]
    fn test_error_response() {
        let resp: ApiResponse<()> = ApiResponse::error(OfsApiCode::NotFound, "entity not found");
        assert_eq!(resp.status, "error");
        assert_eq!(resp.code, OfsApiCode::NotFound);
        assert_eq!(resp.message, "entity not found");
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_into_json() {
        let resp: ApiResponse<String> = ApiResponse::success("data".into());
        let json = resp.into_json();
        assert_eq!(json["status"], "success");
        assert_eq!(json["code"], "OK");
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["a".to_string(), "b".to_string()];
        let page = PaginatedResponse::new(items.clone(), 10, 1, 2);
        assert_eq!(page.data, items);
        assert_eq!(page.total, 10);
        assert_eq!(page.has_more, true);

        let last = PaginatedResponse::new(vec!["z".to_string()], 5, 3, 2);
        assert_eq!(last.has_more, false);
    }
}
