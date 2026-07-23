use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::findings;
use super::{
    identify, payload_hash, DeviceHistory, Finding, Identity, Observation, ObservationResult,
    RiskKind, RiskSeverity, TrackerPolicy,
};

#[derive(Debug)]
struct SensorSeen {
    at_ms: u64,
    zone: Option<String>,
}

#[derive(Debug)]
struct State {
    history: DeviceHistory,
    rssi_sum: i64,
    sensors: BTreeMap<String, SensorSeen>,
    movement_sessions: BTreeSet<String>,
    persistent_reported: bool,
    co_travel_reported: bool,
    disappearance_reported: bool,
    clone_pairs: BTreeSet<String>,
}

#[derive(Debug)]
pub struct Tracker {
    policy: TrackerPolicy,
    devices: BTreeMap<String, State>,
    recent: VecDeque<(u64, String)>,
    last_flood_at_ms: Option<u64>,
}

impl Tracker {
    pub fn new(policy: TrackerPolicy) -> Self {
        Self {
            policy,
            devices: BTreeMap::new(),
            recent: VecDeque::new(),
            last_flood_at_ms: None,
        }
    }

    pub fn policy(&self) -> &TrackerPolicy {
        &self.policy
    }

    pub fn history(&self, identity_key: &str) -> Option<&DeviceHistory> {
        self.devices.get(identity_key).map(|state| &state.history)
    }

    pub fn histories(&self) -> impl Iterator<Item = &DeviceHistory> {
        self.devices.values().map(|state| &state.history)
    }

    pub fn observe(&mut self, observation: Observation) -> ObservationResult {
        let identity = identify(&observation.advertisement);
        let payload = payload_hash(&observation.advertisement);
        let now = observation.monotonic_ms;
        self.prune_recent(now);
        self.recent.push_back((now, identity.key.clone()));

        let allowlisted = contains(&self.policy.allowlisted_identity_keys, &identity.key);
        let policy = self.policy.clone();
        let (mut findings, history) = {
            let state = self.devices.entry(identity.key.clone()).or_insert_with(|| {
                State::new(
                    identity.clone(),
                    now,
                    &payload,
                    observation.advertisement.rssi_dbm,
                )
            });
            let findings = state.observe(&policy, &observation, &payload, allowlisted);
            (findings, state.history.clone())
        };
        if let Some(finding) = self.flood_finding(now) {
            findings.push(finding);
        }

        ObservationResult {
            identity,
            payload_hash: payload,
            history,
            findings,
        }
    }

    pub fn evaluate(&mut self, now_ms: u64) -> Vec<Finding> {
        let mut findings = Vec::new();
        for key in &self.policy.expected_identity_keys {
            let Some(state) = self.devices.get_mut(key) else {
                continue;
            };
            if !state.disappearance_reported
                && now_ms.saturating_sub(state.history.last_seen_ms) >= self.policy.disappearance_ms
            {
                state.disappearance_reported = true;
                findings.push(Finding {
                    kind: RiskKind::Disappeared,
                    severity: RiskSeverity::Warning,
                    identity_key: Some(key.clone()),
                    observed_at_ms: now_ms,
                    summary: "Expected BLE device is no longer observed".to_string(),
                    evidence: vec![format!(
                        "last_seen_ms={}, missing_for_ms={}",
                        state.history.last_seen_ms,
                        now_ms.saturating_sub(state.history.last_seen_ms)
                    )],
                    limitations: vec![
                        "Radio obstruction, scan gaps and device sleep can also explain absence"
                            .to_string(),
                    ],
                });
            }
        }
        findings
    }

    fn prune_recent(&mut self, now_ms: u64) {
        while self
            .recent
            .front()
            .is_some_and(|(at, _)| now_ms.saturating_sub(*at) > self.policy.flood_window_ms)
        {
            self.recent.pop_front();
        }
    }

    fn flood_finding(&mut self, now_ms: u64) -> Option<Finding> {
        let identities: BTreeSet<&str> = self
            .recent
            .iter()
            .map(|(_, identity)| identity.as_str())
            .collect();
        if identities.len() < self.policy.flood_unique_identities
            || self
                .last_flood_at_ms
                .is_some_and(|last| now_ms.saturating_sub(last) <= self.policy.flood_window_ms)
        {
            return None;
        }
        self.last_flood_at_ms = Some(now_ms);
        Some(Finding {
            kind: RiskKind::BeaconFlood,
            severity: RiskSeverity::Warning,
            identity_key: None,
            observed_at_ms: now_ms,
            summary: "Unusually many BLE identities appeared in one scan window".to_string(),
            evidence: vec![format!(
                "unique_identities={}, window_ms={}",
                identities.len(),
                self.policy.flood_window_ms
            )],
            limitations: vec!["Crowded legitimate venues can produce the same pattern".to_string()],
        })
    }
}

impl State {
    fn new(identity: Identity, now: u64, payload: &str, rssi: i16) -> Self {
        Self {
            history: DeviceHistory {
                identity,
                first_seen_ms: now,
                last_seen_ms: now,
                observation_count: 0,
                sensor_count: 0,
                movement_session_count: 0,
                rssi_min_dbm: rssi,
                rssi_max_dbm: rssi,
                rssi_mean_dbm: rssi,
                last_payload_hash: payload.to_string(),
            },
            rssi_sum: 0,
            sensors: BTreeMap::new(),
            movement_sessions: BTreeSet::new(),
            persistent_reported: false,
            co_travel_reported: false,
            disappearance_reported: false,
            clone_pairs: BTreeSet::new(),
        }
    }

    fn observe(
        &mut self,
        policy: &TrackerPolicy,
        observation: &Observation,
        payload: &str,
        allowlisted: bool,
    ) -> Vec<Finding> {
        let now = observation.monotonic_ms;
        let rssi = observation.advertisement.rssi_dbm;
        let mut findings = self.clone_findings(policy, observation);

        self.history.last_seen_ms = now;
        self.history.observation_count = self.history.observation_count.saturating_add(1);
        self.history.rssi_min_dbm = self.history.rssi_min_dbm.min(rssi);
        self.history.rssi_max_dbm = self.history.rssi_max_dbm.max(rssi);
        self.rssi_sum = self.rssi_sum.saturating_add(i64::from(rssi));
        self.history.rssi_mean_dbm = (self.rssi_sum / self.history.observation_count as i64) as i16;
        self.history.last_payload_hash = payload.to_string();
        self.disappearance_reported = false;

        if observation.context.sensor_is_moving {
            if let Some(session) = &observation.context.movement_session {
                self.movement_sessions.insert(session.clone());
            }
        }
        self.sensors.insert(
            observation.context.sensor_id.clone(),
            SensorSeen {
                at_ms: now,
                zone: observation.context.zone.clone(),
            },
        );
        self.history.sensor_count = self.sensors.len();
        self.history.movement_session_count = self.movement_sessions.len();

        if !allowlisted
            && !self.persistent_reported
            && now.saturating_sub(self.history.first_seen_ms) >= policy.persistent_unknown_ms
        {
            self.persistent_reported = true;
            findings.push(findings::persistent(&self.history, now));
        }
        if !allowlisted
            && !self.co_travel_reported
            && self.movement_sessions.len() >= policy.co_travel_min_sessions
        {
            self.co_travel_reported = true;
            findings.push(findings::co_travel(&self.history, now));
        }
        findings
    }

    fn clone_findings(
        &mut self,
        policy: &TrackerPolicy,
        observation: &Observation,
    ) -> Vec<Finding> {
        if !self.history.identity.confidence.supports_clone_evidence() {
            return Vec::new();
        }
        let Some(zone) = observation.context.zone.as_deref() else {
            return Vec::new();
        };
        for (sensor, previous) in &self.sensors {
            let Some(previous_zone) = previous.zone.as_deref() else {
                continue;
            };
            if sensor == &observation.context.sensor_id
                || previous_zone == zone
                || observation.monotonic_ms.abs_diff(previous.at_ms) > policy.clone_window_ms
            {
                continue;
            }
            let pair = ordered_pair(sensor, &observation.context.sensor_id);
            if self.clone_pairs.insert(pair) {
                return vec![Finding {
                    kind: RiskKind::PossibleClone,
                    severity: RiskSeverity::High,
                    identity_key: Some(self.history.identity.key.clone()),
                    observed_at_ms: observation.monotonic_ms,
                    summary: "Strong BLE identity was observed in two zones concurrently"
                        .to_string(),
                    evidence: vec![format!(
                        "zones={previous_zone},{zone}, separation_ms={}",
                        observation.monotonic_ms.abs_diff(previous.at_ms)
                    )],
                    limitations: vec![
                        "Sensor clocks and zone assignment must be trustworthy".to_string(),
                        "This is clone evidence, not proof of malicious intent".to_string(),
                    ],
                }];
            }
        }
        Vec::new()
    }
}

fn contains(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn ordered_pair(left: &str, right: &str) -> String {
    if left <= right {
        format!("{left}\u{0}{right}")
    } else {
        format!("{right}\u{0}{left}")
    }
}
