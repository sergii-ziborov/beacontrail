use alloc::format;
use alloc::string::ToString;
use alloc::vec;

use super::{DeviceHistory, Finding, RiskKind, RiskSeverity};

pub(super) fn persistent(history: &DeviceHistory, now: u64) -> Finding {
    Finding {
        kind: RiskKind::PersistentUnknown,
        severity: RiskSeverity::Info,
        identity_key: Some(history.identity.key.clone()),
        observed_at_ms: now,
        summary: "Unknown BLE identity persisted beyond the configured threshold".to_string(),
        evidence: vec![format!(
            "dwell_ms={}, observations={}",
            now.saturating_sub(history.first_seen_ms),
            history.observation_count
        )],
        limitations: vec!["Persistence alone does not imply tracking or danger".to_string()],
    }
}

pub(super) fn co_travel(history: &DeviceHistory, now: u64) -> Finding {
    Finding {
        kind: RiskKind::CoTravel,
        severity: RiskSeverity::Warning,
        identity_key: Some(history.identity.key.clone()),
        observed_at_ms: now,
        summary: "Unknown BLE identity recurred across moving sensor sessions".to_string(),
        evidence: vec![format!(
            "movement_sessions={}",
            history.movement_session_count
        )],
        limitations: vec![
            "The caller must define independent movement sessions correctly".to_string(),
            "Generic BLE evidence cannot identify an owner or intent".to_string(),
        ],
    }
}
