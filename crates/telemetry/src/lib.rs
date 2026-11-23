use chrono::Utc;
use lingxi_domain::{ProviderKind, ProviderQuotaSnapshot, ProviderStatus, ResourceUsageSnapshot};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("telemetry collection error: {0}")]
    Collection(String),
}

pub fn sample_process_resources() -> ResourceUsageSnapshot {
    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::everything(),
    );
    if let Some(proc_info) = sys.process(pid) {
        ResourceUsageSnapshot {
            cpu_percent: proc_info.cpu_usage(),
            memory_rss_bytes: proc_info.memory(),
            child_process_count: 0,
        }
    } else {
        ResourceUsageSnapshot::default()
    }
}

/// Builds honest quota snapshots from detected provider statuses.
/// Never fabricates remaining quota percentages or pool balances when a provider
/// has no real local quota feed.
pub fn build_quota_snapshots(statuses: &[ProviderStatus]) -> Vec<ProviderQuotaSnapshot> {
    let tracked = [
        ProviderKind::Claude,
        ProviderKind::Codex,
        ProviderKind::Antigravity,
        ProviderKind::Cursor,
        ProviderKind::Grok,
        ProviderKind::Muse,
    ];
    let now = Utc::now();
    tracked
        .into_iter()
        .map(|kind| {
            let installed = statuses.iter().any(|s| {
                s.kind == kind && s.install_state == lingxi_domain::ProviderInstallState::Installed
            });
            let note = if installed {
                "CLI installed; live quota API not queried in offline mode".to_string()
            } else {
                "Provider not installed on this machine".to_string()
            };
            ProviderQuotaSnapshot {
                provider: kind,
                available: installed,
                source_note: note,
                pools: Vec::new(),
                updated_at: now,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_resources_reports_non_zero_memory() {
        let snap = sample_process_resources();
        assert!(snap.memory_rss_bytes > 0);
    }

    #[test]
    fn quota_snapshots_cover_6_platforms_without_fabricating_pools() {
        let snaps = build_quota_snapshots(&[]);
        assert_eq!(snaps.len(), 6);
        assert!(snaps.iter().all(|s| !s.available && s.pools.is_empty()));
    }
}
