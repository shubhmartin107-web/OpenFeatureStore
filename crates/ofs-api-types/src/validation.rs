use crate::response::ApiErrorDetail;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub reason: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

impl From<ValidationError> for ApiErrorDetail {
    fn from(e: ValidationError) -> Self {
        ApiErrorDetail {
            field: Some(e.field),
            reason: e.reason,
        }
    }
}

pub trait Validate {
    fn validate(&self) -> Result<(), Vec<ValidationError>>;
}

pub fn validate_entity_key(key: &str) -> Result<(), ValidationError> {
    if key.is_empty() {
        return Err(ValidationError::new("entity_key", "must not be empty"));
    }
    if key.len() > 1024 {
        return Err(ValidationError::new(
            "entity_key",
            "must not exceed 1024 characters",
        ));
    }
    Ok(())
}

pub fn validate_feature_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::new("feature_name", "must not be empty"));
    }
    if name.len() > 256 {
        return Err(ValidationError::new(
            "feature_name",
            "must not exceed 256 characters",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(ValidationError::new(
            "feature_name",
            "must contain only alphanumeric, underscore, hyphen, or dot characters",
        ));
    }
    Ok(())
}

pub fn validate_project_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        return Err(ValidationError::new("project", "must not be empty"));
    }
    if name.len() > 128 {
        return Err(ValidationError::new(
            "project",
            "must not exceed 128 characters",
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ValidationError::new(
            "project",
            "must contain only alphanumeric, underscore, or hyphen characters",
        ));
    }
    Ok(())
}

pub fn validate_entity_keys(keys: &[String]) -> Result<(), Vec<ValidationError>> {
    if keys.is_empty() {
        return Err(vec![ValidationError::new(
            "entity_keys",
            "must contain at least one key",
        )]);
    }
    if keys.len() > 1000 {
        return Err(vec![ValidationError::new(
            "entity_keys",
            "must not exceed 1000 keys per request",
        )]);
    }
    let mut errors = Vec::new();
    for key in keys {
        if let Err(e) = validate_entity_key(key) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn validate_feature_names(names: &[String]) -> Result<(), Vec<ValidationError>> {
    if names.is_empty() {
        return Err(vec![ValidationError::new(
            "feature_names",
            "must contain at least one feature",
        )]);
    }
    let mut errors = Vec::new();
    for name in names {
        if let Err(e) = validate_feature_name(name) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_entity_key_ok() {
        assert!(validate_entity_key("user:123").is_ok());
    }

    #[test]
    fn test_validate_entity_key_empty() {
        assert!(validate_entity_key("").is_err());
    }

    #[test]
    fn test_validate_entity_key_too_long() {
        let long = "a".repeat(1025);
        assert!(validate_entity_key(&long).is_err());
    }

    #[test]
    fn test_validate_feature_name_ok() {
        assert!(validate_feature_name("click_rate_p90").is_ok());
    }

    #[test]
    fn test_validate_feature_name_invalid_chars() {
        assert!(validate_feature_name("click rate!").is_err());
    }

    #[test]
    fn test_validate_project_name_ok() {
        assert!(validate_project_name("my-project").is_ok());
    }

    #[test]
    fn test_validate_project_name_invalid() {
        assert!(validate_project_name("my project!").is_err());
    }

    #[test]
    fn test_validate_entity_keys_empty() {
        assert!(validate_entity_keys(&[]).is_err());
    }

    #[test]
    fn test_validate_entity_keys_too_many() {
        let keys: Vec<String> = (0..1001).map(|i| format!("key-{}", i)).collect();
        assert!(validate_entity_keys(&keys).is_err());
    }

    #[test]
    fn test_validate_entity_keys_invalid_content() {
        let keys = vec!["".into(), "valid".into()];
        assert!(validate_entity_keys(&keys).is_err());
    }

    #[test]
    fn test_validation_error_conversion() {
        let err = ValidationError::new("field", "something wrong");
        let detail: ApiErrorDetail = err.into();
        assert_eq!(detail.field, Some("field".into()));
        assert_eq!(detail.reason, "something wrong");
    }
}
