use sutra_schema::{ComponentHealth, HealthStatus};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks health status of components and provides liveness/readiness checks.
#[derive(Debug)]
pub struct HealthRegistry {
    components: HashMap<String, ComponentEntry>,
}

#[derive(Debug, Clone)]
struct ComponentEntry {
    initial_status: HealthStatus,
    last_heartbeat: Instant,
    message: Option<String>,
}

impl HealthRegistry {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, initial_status: HealthStatus) {
        self.components.insert(
            name.to_owned(),
            ComponentEntry {
                initial_status,
                last_heartbeat: Instant::now(),
                message: None,
            },
        );
    }

    pub fn heartbeat(&mut self, name: &str) {
        if let Some(entry) = self.components.get_mut(name) {
            entry.last_heartbeat = Instant::now();
        }
    }

    pub fn set_status(&mut self, name: &str, status: HealthStatus, message: Option<&str>) {
        if let Some(entry) = self.components.get_mut(name) {
            entry.initial_status = status;
            entry.message = message.map(String::from);
            entry.last_heartbeat = Instant::now();
        }
    }

    pub fn status(&self, name: &str) -> Option<ComponentHealth> {
        self.components.get(name).map(|entry| ComponentHealth {
            name: name.to_owned(),
            status: entry.initial_status,
            message: entry.message.clone(),
            last_heartbeat_ms: entry.last_heartbeat.elapsed().as_millis() as u64,
        })
    }

    pub fn all_statuses(&self) -> Vec<ComponentHealth> {
        let mut result: Vec<ComponentHealth> = self
            .components
            .iter()
            .map(|(name, entry)| ComponentHealth {
                name: name.clone(),
                status: entry.initial_status,
                message: entry.message.clone(),
                last_heartbeat_ms: entry.last_heartbeat.elapsed().as_millis() as u64,
            })
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    pub fn is_healthy(&self) -> bool {
        self.components
            .values()
            .all(|e| e.initial_status == HealthStatus::Healthy || e.initial_status == HealthStatus::Degraded)
    }

    pub fn is_ready(&self) -> bool {
        self.components
            .values()
            .all(|e| e.initial_status == HealthStatus::Healthy)
    }

    pub fn unhealthy_components(&self) -> Vec<String> {
        self.components
            .iter()
            .filter(|(_, e)| e.initial_status == HealthStatus::Unhealthy)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn has_recent_heartbeat(&self, name: &str, max_age: Duration) -> bool {
        self.components
            .get(name)
            .map(|e| e.last_heartbeat.elapsed() < max_age)
            .unwrap_or(false)
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
    use sutra_schema::HealthStatus;

    #[test]
    fn test_health_registry_new_is_empty() {
        let reg = HealthRegistry::new();
        assert!(reg.all_statuses().is_empty());
        assert!(reg.is_healthy());
        assert!(reg.is_ready());
    }

    #[test]
    fn test_register_component() {
        let mut reg = HealthRegistry::new();
        reg.register("mgtg", HealthStatus::Healthy);
        let statuses = reg.all_statuses();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "mgtg");
        assert_eq!(statuses[0].status, HealthStatus::Healthy);
    }

    #[test]
    fn test_status_returns_none_for_unknown() {
        let reg = HealthRegistry::new();
        assert!(reg.status("unknown").is_none());
    }

    #[test]
    fn test_set_status_and_message() {
        let mut reg = HealthRegistry::new();
        reg.register("ml", HealthStatus::Healthy);
        reg.set_status("ml", HealthStatus::Unhealthy, Some("model not loaded"));
        let status = reg.status("ml").unwrap();
        assert_eq!(status.status, HealthStatus::Unhealthy);
        assert_eq!(status.message, Some("model not loaded".into()));
    }

    #[test]
    fn test_heartbeat_updates_timestamp() {
        let mut reg = HealthRegistry::new();
        reg.register("mgtg", HealthStatus::Healthy);
        // Wait so the elapsed time becomes measurable
        std::thread::sleep(Duration::from_millis(5));
        let before = reg.status("mgtg").unwrap().last_heartbeat_ms;
        assert!(before >= 1, "elapsed should be measurable: {before}ms");
        reg.heartbeat("mgtg");
        let after = reg.status("mgtg").unwrap().last_heartbeat_ms;
        assert!(
            after < before,
            "heartbeat should reset elapsed time: before={before}ms, after={after}ms"
        );
    }

    #[test]
    fn test_is_healthy_true_when_all_healthy() {
        let mut reg = HealthRegistry::new();
        reg.register("a", HealthStatus::Healthy);
        reg.register("b", HealthStatus::Healthy);
        assert!(reg.is_healthy());
    }

    #[test]
    fn test_is_healthy_true_with_degraded() {
        let mut reg = HealthRegistry::new();
        reg.register("a", HealthStatus::Healthy);
        reg.register("b", HealthStatus::Degraded);
        assert!(reg.is_healthy());
    }

    #[test]
    fn test_is_healthy_false_with_unhealthy() {
        let mut reg = HealthRegistry::new();
        reg.register("a", HealthStatus::Healthy);
        reg.register("b", HealthStatus::Unhealthy);
        assert!(!reg.is_healthy());
    }

    #[test]
    fn test_is_ready_requires_all_healthy() {
        let mut reg = HealthRegistry::new();
        reg.register("a", HealthStatus::Healthy);
        assert!(reg.is_ready());
        reg.register("b", HealthStatus::Degraded);
        assert!(!reg.is_ready());
    }

    #[test]
    fn test_unhealthy_components() {
        let mut reg = HealthRegistry::new();
        reg.register("a", HealthStatus::Healthy);
        reg.register("b", HealthStatus::Unhealthy);
        reg.register("c", HealthStatus::Degraded);
        reg.register("d", HealthStatus::Unhealthy);
        let unhealthy = reg.unhealthy_components();
        assert_eq!(unhealthy.len(), 2);
        assert!(unhealthy.contains(&"b".to_owned()));
        assert!(unhealthy.contains(&"d".to_owned()));
    }

    #[test]
    fn test_has_recent_heartbeat() {
        let mut reg = HealthRegistry::new();
        reg.register("mgtg", HealthStatus::Healthy);
        assert!(reg.has_recent_heartbeat("mgtg", Duration::from_secs(60)));
        assert!(!reg.has_recent_heartbeat("unknown", Duration::from_secs(60)));
    }

    #[test]
    fn test_heartbeat_on_unregistered_does_nothing() {
        let mut reg = HealthRegistry::new();
        reg.heartbeat("nonexistent"); // should not panic
    }

    #[test]
    fn test_set_status_on_unregistered_does_nothing() {
        let mut reg = HealthRegistry::new();
        reg.set_status("nonexistent", HealthStatus::Unhealthy, None); // should not panic
    }

    #[test]
    fn test_register_duplicate_overwrites() {
        let mut reg = HealthRegistry::new();
        reg.register("mgtg", HealthStatus::Healthy);
        reg.register("mgtg", HealthStatus::Unhealthy);
        assert_eq!(reg.unhealthy_components().len(), 1);
    }

    #[test]
    fn test_all_statuses_sorted() {
        let mut reg = HealthRegistry::new();
        reg.register("z", HealthStatus::Healthy);
        reg.register("a", HealthStatus::Healthy);
        reg.register("m", HealthStatus::Healthy);
        let names: Vec<String> = reg.all_statuses().into_iter().map(|h| h.name).collect();
        assert_eq!(names, vec!["a".to_owned(), "m".to_owned(), "z".to_owned()]);
    }
}
