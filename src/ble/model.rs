use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressType {
    Public,
    RandomStatic,
    ResolvablePrivate,
    NonResolvablePrivate,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManufacturerData {
    pub company_id: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceData {
    pub uuid: String,
    pub data: Vec<u8>,
}

/// One received BLE advertisement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advertisement {
    pub address: String,
    pub address_type: AddressType,
    pub local_name: Option<String>,
    pub rssi_dbm: i16,
    pub tx_power_dbm: Option<i16>,
    pub connectable: Option<bool>,
    #[serde(default)]
    pub service_uuids: Vec<String>,
    #[serde(default)]
    pub manufacturer_data: Vec<ManufacturerData>,
    #[serde(default)]
    pub service_data: Vec<ServiceData>,
    /// Stable identity decoded by a protocol-specific adapter, when available.
    /// It is hashed before entering history or chronicle records.
    pub protocol_identity: Option<String>,
}

/// Context supplied by the sensor, never inferred from RSSI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorContext {
    pub sensor_id: String,
    /// Caller-owned physical or logical zone used for concurrent-clone evidence.
    pub zone: Option<String>,
    /// Caller-owned movement segment. Repeated sightings across distinct moving
    /// segments are the minimum evidence for a co-travel warning.
    pub movement_session: Option<String>,
    pub sensor_is_moving: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub monotonic_ms: u64,
    pub unix_epoch_ms: Option<i64>,
    pub context: SensorContext,
    pub advertisement: Advertisement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityConfidence {
    Protocol,
    CallerProvided,
    StaticAddress,
    EphemeralAddress,
}

impl IdentityConfidence {
    pub fn supports_clone_evidence(self) -> bool {
        matches!(self, Self::Protocol | Self::CallerProvided)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Opaque deterministic key. It is a fingerprint, not a cryptographic
    /// anonymization guarantee.
    pub key: String,
    pub confidence: IdentityConfidence,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceHistory {
    pub identity: Identity,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    pub observation_count: u64,
    pub sensor_count: usize,
    pub movement_session_count: usize,
    pub rssi_min_dbm: i16,
    pub rssi_max_dbm: i16,
    pub rssi_mean_dbm: i16,
    pub last_payload_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskKind {
    PersistentUnknown,
    CoTravel,
    Disappeared,
    PossibleClone,
    BeaconFlood,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskSeverity {
    Info,
    Warning,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: RiskKind,
    pub severity: RiskSeverity,
    pub identity_key: Option<String>,
    pub observed_at_ms: u64,
    pub summary: String,
    pub evidence: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationResult {
    pub identity: Identity,
    pub payload_hash: String,
    pub history: DeviceHistory,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerPolicy {
    pub persistent_unknown_ms: u64,
    pub co_travel_min_sessions: usize,
    pub disappearance_ms: u64,
    pub clone_window_ms: u64,
    pub flood_window_ms: u64,
    pub flood_unique_identities: usize,
    #[serde(default)]
    pub allowlisted_identity_keys: Vec<String>,
    #[serde(default)]
    pub expected_identity_keys: Vec<String>,
}

impl Default for TrackerPolicy {
    fn default() -> Self {
        Self {
            persistent_unknown_ms: 30 * 60 * 1000,
            co_travel_min_sessions: 3,
            disappearance_ms: 5 * 60 * 1000,
            clone_window_ms: 10_000,
            flood_window_ms: 60_000,
            flood_unique_identities: 100,
            allowlisted_identity_keys: Vec::new(),
            expected_identity_keys: Vec::new(),
        }
    }
}

pub trait Collector {
    type Error;

    fn scan(&mut self, output: &mut Vec<Advertisement>) -> Result<(), Self::Error>;
}

#[derive(Debug, Default)]
pub struct Snapshot {
    pub advertisements: Vec<Advertisement>,
}

impl Snapshot {
    pub const fn new() -> Self {
        Self {
            advertisements: Vec::new(),
        }
    }

    pub fn refresh<C: Collector>(&mut self, collector: &mut C) -> Result<(), C::Error> {
        self.advertisements.clear();
        collector.scan(&mut self.advertisements)
    }
}
