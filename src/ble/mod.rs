//! Portable Bluetooth Low Energy observations, history and risk evidence.
//!
//! The module is `no_std + alloc`. A platform or firmware adapter supplies
//! advertisements through [`Collector`]; RadioChron supplies normalized
//! models, protocol-aware identity fingerprints and stateful detectors.
//!
//! RSSI is signal evidence, not a distance measurement. Generic private
//! addresses can rotate, so only decoded protocol identities or caller-supplied
//! identities are treated as strong enough for clone detection.

mod findings;
mod fingerprint;
mod model;
mod tracker;

pub use fingerprint::{identify, payload_hash};
pub use model::{
    AddressType, Advertisement, Collector, DeviceHistory, Finding, Identity, IdentityConfidence,
    ManufacturerData, Observation, ObservationResult, RiskKind, RiskSeverity, SensorContext,
    ServiceData, Snapshot, TrackerPolicy,
};
pub use tracker::Tracker;

#[cfg(test)]
mod tests;
