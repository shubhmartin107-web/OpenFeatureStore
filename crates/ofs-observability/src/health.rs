use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Degraded(String),
    Unavailable(String),
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ok | Self::Degraded(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: &'static str,
    pub status: HealthStatus,
}

impl HealthCheck {
    pub fn new(name: &'static str, status: HealthStatus) -> Self {
        Self { name, status }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub uptime_seconds: u64,
    pub version: &'static str,
}

impl HealthReport {
    pub fn new(checks: Vec<HealthCheck>, start_time: std::time::Instant) -> Self {
        let uptime = start_time.elapsed().as_secs();
        let overall = if checks.iter().all(|c| c.status.is_healthy()) {
            HealthStatus::Ok
        } else if checks
            .iter()
            .any(|c| matches!(c.status, HealthStatus::Unavailable(_)))
        {
            HealthStatus::Unavailable("one or more checks failed".into())
        } else {
            HealthStatus::Degraded("one or more checks degraded".into())
        };

        Self {
            status: overall,
            checks,
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

pub type HealthCheckFn = Box<dyn Fn() -> HealthStatus + Send + Sync>;

pub struct HealthRegistry {
    checks: Vec<(&'static str, HealthCheckFn)>,
}

impl HealthRegistry {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn register(&mut self, name: &'static str, check: HealthCheckFn) {
        self.checks.push((name, check));
    }

    pub fn report(&self, start_time: std::time::Instant) -> HealthReport {
        let checks: Vec<HealthCheck> = self
            .checks
            .iter()
            .map(|(name, check)| HealthCheck::new(name, check()))
            .collect();
        HealthReport::new(checks, start_time)
    }
}

impl Default for HealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_health_status_is_healthy() {
        assert!(HealthStatus::Ok.is_healthy());
        assert!(!HealthStatus::Degraded("slow".into()).is_healthy());
        assert!(!HealthStatus::Unavailable("down".into()).is_healthy());
    }

    #[test]
    fn test_health_report_all_ok() {
        let start = Instant::now();
        let report = HealthReport::new(vec![HealthCheck::new("test", HealthStatus::Ok)], start);
        assert_eq!(report.status, HealthStatus::Ok);
        let _ = report.uptime_seconds;
    }

    #[test]
    fn test_health_report_degraded() {
        let start = Instant::now();
        let report = HealthReport::new(
            vec![
                HealthCheck::new("check1", HealthStatus::Ok),
                HealthCheck::new("check2", HealthStatus::Degraded("slow".into())),
            ],
            start,
        );
        assert!(matches!(report.status, HealthStatus::Degraded(_)));
        assert!(report.status.is_ready());
    }

    #[test]
    fn test_health_registry() {
        let start = Instant::now();
        let mut registry = HealthRegistry::new();
        registry.register("test", Box::new(|| HealthStatus::Ok));
        let report = registry.report(start);
        assert_eq!(report.checks.len(), 1);
        assert!(report.status.is_healthy());
    }
}
