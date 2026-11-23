use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::provider::ProviderKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContextBreakdown {
    pub system_tokens: u64,
    pub tools_tokens: u64,
    pub messages_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub context_window_limit: u64,
}

impl ContextBreakdown {
    pub fn total_used(&self) -> u64 {
        self.system_tokens + self.tools_tokens + self.messages_tokens
    }

    pub fn utilization_ratio(&self) -> f64 {
        if self.context_window_limit == 0 {
            return 0.0;
        }
        (self.total_used() as f64 / self.context_window_limit as f64).min(1.0)
    }

    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_read_tokens + self.messages_tokens;
        if total == 0 {
            return 0.0;
        }
        self.cache_read_tokens as f64 / total as f64
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaPoolEntry {
    pub pool_name: String,
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<DateTime<Utc>>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderQuotaSnapshot {
    pub provider: ProviderKind,
    pub available: bool,
    pub source_note: String,
    pub pools: Vec<QuotaPoolEntry>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceUsageSnapshot {
    pub cpu_percent: f32,
    pub memory_rss_bytes: u64,
    pub child_process_count: usize,
}
