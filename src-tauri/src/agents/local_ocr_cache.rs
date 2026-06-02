use std::collections::HashMap;

pub const HEALTHY_TTL_MS: u64 = 5_000;
pub const UNHEALTHY_TTL_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHealthStatus {
    pub healthy: bool,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
}

impl LocalHealthStatus {
    pub fn unhealthy() -> Self {
        Self {
            healthy: false,
            version: None,
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct LocalHealthEntry {
    checked_at_ms: u64,
    status: LocalHealthStatus,
}

#[derive(Debug, Default)]
pub struct LocalHealthCache {
    entries: HashMap<String, LocalHealthEntry>,
}

impl LocalHealthCache {
    pub fn get(&self, url: &str, now_ms: u64) -> Option<LocalHealthStatus> {
        let entry = self.entries.get(url)?;
        let ttl = if entry.status.healthy {
            HEALTHY_TTL_MS
        } else {
            UNHEALTHY_TTL_MS
        };
        if now_ms.saturating_sub(entry.checked_at_ms) <= ttl {
            Some(entry.status.clone())
        } else {
            None
        }
    }

    pub fn update(&mut self, url: &str, status: LocalHealthStatus, now_ms: u64) {
        self.entries.insert(
            url.to_string(),
            LocalHealthEntry {
                checked_at_ms: now_ms,
                status,
            },
        );
    }
}
