use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use super::*;

fn advertisement(address: &str) -> Advertisement {
    Advertisement {
        address: address.to_string(),
        address_type: AddressType::RandomStatic,
        local_name: Some("tag".to_string()),
        rssi_dbm: -60,
        tx_power_dbm: Some(-20),
        connectable: Some(false),
        service_uuids: Vec::new(),
        manufacturer_data: Vec::new(),
        service_data: Vec::new(),
        protocol_identity: None,
    }
}

fn observation(address: &str, at: u64, session: Option<&str>) -> Observation {
    Observation {
        monotonic_ms: at,
        unix_epoch_ms: None,
        context: SensorContext {
            sensor_id: "sensor-a".to_string(),
            zone: Some("front".to_string()),
            movement_session: session.map(ToString::to_string),
            sensor_is_moving: session.is_some(),
        },
        advertisement: advertisement(address),
    }
}

#[test]
fn protocol_identity_survives_private_address_rotation() {
    let mut first = advertisement("aa");
    first.address_type = AddressType::ResolvablePrivate;
    let mut ibeacon = vec![0x02, 0x15];
    ibeacon.extend_from_slice(&[7; 16]);
    ibeacon.extend_from_slice(&[0, 1, 0, 2, 0xc5]);
    first.manufacturer_data.push(ManufacturerData {
        company_id: 0x004c,
        data: ibeacon,
    });
    let mut second = first.clone();
    second.address = "bb".to_string();

    let one = identify(&first);
    let two = identify(&second);
    assert_eq!(one.key, two.key);
    assert_eq!(one.confidence, IdentityConfidence::Protocol);
    assert_eq!(one.protocol.as_deref(), Some("ibeacon"));
}

#[test]
fn private_address_without_protocol_identity_is_ephemeral() {
    let mut item = advertisement("aa");
    item.address_type = AddressType::ResolvablePrivate;
    assert_eq!(
        identify(&item).confidence,
        IdentityConfidence::EphemeralAddress
    );
}

#[test]
fn reports_persistence_and_co_travel_only_after_evidence_thresholds() {
    let mut tracker = Tracker::new(TrackerPolicy {
        persistent_unknown_ms: 100,
        co_travel_min_sessions: 2,
        ..TrackerPolicy::default()
    });
    assert!(tracker
        .observe(observation("aa", 0, Some("walk-1")))
        .findings
        .is_empty());
    let result = tracker.observe(observation("aa", 100, Some("walk-2")));

    assert!(result
        .findings
        .iter()
        .any(|finding| finding.kind == RiskKind::PersistentUnknown));
    assert!(result
        .findings
        .iter()
        .any(|finding| finding.kind == RiskKind::CoTravel));
    assert_eq!(result.history.movement_session_count, 2);
}

#[test]
fn allowlist_suppresses_unknown_and_co_travel_findings() {
    let first = observation("aa", 0, Some("walk-1"));
    let key = identify(&first.advertisement).key;
    let mut tracker = Tracker::new(TrackerPolicy {
        persistent_unknown_ms: 0,
        co_travel_min_sessions: 1,
        allowlisted_identity_keys: vec![key],
        ..TrackerPolicy::default()
    });

    assert!(tracker.observe(first).findings.is_empty());
}

#[test]
fn clone_evidence_requires_strong_identity_and_distinct_zones() {
    let mut first = observation("aa", 10, None);
    first.advertisement.protocol_identity = Some("asset-7".to_string());
    let mut second = first.clone();
    second.monotonic_ms = 15;
    second.context.sensor_id = "sensor-b".to_string();
    second.context.zone = Some("back".to_string());

    let mut tracker = Tracker::new(TrackerPolicy::default());
    tracker.observe(first);
    let findings = tracker.observe(second).findings;
    assert_eq!(findings[0].kind, RiskKind::PossibleClone);
}

#[test]
fn expected_device_disappearance_is_evaluated_explicitly() {
    let first = observation("aa", 10, None);
    let key = identify(&first.advertisement).key;
    let mut tracker = Tracker::new(TrackerPolicy {
        disappearance_ms: 50,
        expected_identity_keys: vec![key],
        ..TrackerPolicy::default()
    });
    tracker.observe(first);

    assert!(tracker.evaluate(59).is_empty());
    assert_eq!(tracker.evaluate(60)[0].kind, RiskKind::Disappeared);
    assert!(tracker.evaluate(100).is_empty());
}

#[test]
fn flood_detector_counts_unique_identities_inside_window() {
    let mut tracker = Tracker::new(TrackerPolicy {
        flood_window_ms: 100,
        flood_unique_identities: 3,
        ..TrackerPolicy::default()
    });
    tracker.observe(observation("aa", 1, None));
    tracker.observe(observation("bb", 2, None));
    let result = tracker.observe(observation("cc", 3, None));

    assert!(result
        .findings
        .iter()
        .any(|finding| finding.kind == RiskKind::BeaconFlood));
}

#[test]
fn policy_deserializes_as_a_partial_configuration() {
    let policy: TrackerPolicy = serde_json::from_str(r#"{"persistent_unknown_ms":25}"#).unwrap();

    assert_eq!(policy.persistent_unknown_ms, 25);
    assert_eq!(
        policy.co_travel_min_sessions,
        TrackerPolicy::default().co_travel_min_sessions
    );
}
