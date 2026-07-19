use ofs_core::errors::OfsResult;
use std::collections::HashMap;

use crate::authn::AuthIdentity;

/// Permissions that can be checked against an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Admin,
}

/// RBAC checker that validates access against roles and project-level permissions.
pub struct RbacChecker {
    /// Global role → permissions mapping.
    role_permissions: HashMap<String, Vec<Permission>>,
    /// Project-specific role → permissions mapping.
    project_role_permissions: HashMap<String, Vec<Permission>>,
}

impl Default for RbacChecker {
    fn default() -> Self {
        let mut role_permissions = HashMap::new();
        role_permissions.insert(
            "admin".to_string(),
            vec![Permission::Read, Permission::Write, Permission::Admin],
        );
        role_permissions.insert(
            "write".to_string(),
            vec![Permission::Read, Permission::Write],
        );
        role_permissions.insert("read".to_string(), vec![Permission::Read]);

        let mut project_role_permissions = HashMap::new();
        project_role_permissions.insert(
            "admin".to_string(),
            vec![Permission::Read, Permission::Write, Permission::Admin],
        );
        project_role_permissions.insert(
            "writer".to_string(),
            vec![Permission::Read, Permission::Write],
        );
        project_role_permissions.insert("reader".to_string(), vec![Permission::Read]);

        Self {
            role_permissions,
            project_role_permissions,
        }
    }
}

impl RbacChecker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an identity has the required permission for a given project.
    pub fn check_access(
        &self,
        identity: &AuthIdentity,
        project: &str,
        required: Permission,
    ) -> OfsResult<()> {
        // Check global roles
        for role in &identity.roles {
            if let Some(perms) = self.role_permissions.get(role)
                && (perms.contains(&required) || perms.contains(&Permission::Admin))
            {
                return Ok(());
            }
        }

        // Check project-specific roles
        if let Some(project_role) = identity.project_roles.get(project)
            && let Some(perms) = self.project_role_permissions.get(project_role)
            && (perms.contains(&required) || perms.contains(&Permission::Admin))
        {
            return Ok(());
        }

        Err(ofs_core::errors::OfsError::Forbidden(format!(
            "Identity '{}' lacks {:?} permission on project '{}'",
            identity.subject, required, project
        )))
    }

    /// Check if identity has admin access (any project).
    pub fn is_admin(&self, identity: &AuthIdentity) -> bool {
        identity.roles.contains(&"admin".to_string())
            || identity.project_roles.values().any(|r| r == "admin")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin_identity() -> AuthIdentity {
        AuthIdentity {
            subject: "admin-user".to_string(),
            roles: vec!["admin".to_string()],
            project_roles: HashMap::new(),
        }
    }

    fn reader_identity() -> AuthIdentity {
        AuthIdentity {
            subject: "reader-user".to_string(),
            roles: vec!["read".to_string()],
            project_roles: HashMap::new(),
        }
    }

    fn project_reader_identity() -> AuthIdentity {
        let mut project_roles = HashMap::new();
        project_roles.insert("project-a".to_string(), "reader".to_string());
        AuthIdentity {
            subject: "project-reader".to_string(),
            roles: vec![],
            project_roles,
        }
    }

    #[test]
    fn test_admin_has_read_access() {
        let rbac = RbacChecker::new();
        assert!(
            rbac.check_access(&admin_identity(), "any-project", Permission::Read)
                .is_ok()
        );
    }

    #[test]
    fn test_admin_has_write_access() {
        let rbac = RbacChecker::new();
        assert!(
            rbac.check_access(&admin_identity(), "any-project", Permission::Write)
                .is_ok()
        );
    }

    #[test]
    fn test_reader_cannot_write() {
        let rbac = RbacChecker::new();
        let result = rbac.check_access(&reader_identity(), "any-project", Permission::Write);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_can_read() {
        let rbac = RbacChecker::new();
        assert!(
            rbac.check_access(&reader_identity(), "any-project", Permission::Read)
                .is_ok()
        );
    }

    #[test]
    fn test_project_reader_can_read_project() {
        let rbac = RbacChecker::new();
        assert!(
            rbac.check_access(&project_reader_identity(), "project-a", Permission::Read)
                .is_ok()
        );
    }

    #[test]
    fn test_project_reader_cannot_write_other_project() {
        let rbac = RbacChecker::new();
        let result = rbac.check_access(&project_reader_identity(), "project-b", Permission::Read);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_admin() {
        let rbac = RbacChecker::new();
        assert!(rbac.is_admin(&admin_identity()));
        assert!(!rbac.is_admin(&reader_identity()));
    }
}
