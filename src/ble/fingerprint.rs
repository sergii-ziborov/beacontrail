use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::{AddressType, Advertisement, Identity, IdentityConfidence};

pub fn identify(advertisement: &Advertisement) -> Identity {
    if let Some(value) = advertisement.protocol_identity.as_deref() {
        return identity(
            "caller",
            value.as_bytes(),
            IdentityConfidence::CallerProvided,
        );
    }
    if let Some(bytes) = ibeacon_identity(advertisement) {
        return identity("ibeacon", &bytes, IdentityConfidence::Protocol);
    }
    if let Some(bytes) = eddystone_uid(advertisement) {
        return identity("eddystone_uid", &bytes, IdentityConfidence::Protocol);
    }

    let confidence = match advertisement.address_type {
        AddressType::Public | AddressType::RandomStatic => IdentityConfidence::StaticAddress,
        AddressType::ResolvablePrivate
        | AddressType::NonResolvablePrivate
        | AddressType::Unknown => IdentityConfidence::EphemeralAddress,
    };
    identity("address", advertisement.address.as_bytes(), confidence)
}

pub fn payload_hash(advertisement: &Advertisement) -> String {
    let mut bytes = Vec::new();
    push_text(
        &mut bytes,
        advertisement.local_name.as_deref().unwrap_or(""),
    );
    let mut services = advertisement.service_uuids.clone();
    services.sort();
    for uuid in services {
        push_text(&mut bytes, &normalized_uuid(&uuid));
    }
    for item in &advertisement.manufacturer_data {
        bytes.extend_from_slice(&item.company_id.to_le_bytes());
        bytes.extend_from_slice(&item.data);
    }
    for item in &advertisement.service_data {
        push_text(&mut bytes, &normalized_uuid(&item.uuid));
        bytes.extend_from_slice(&item.data);
    }
    format!("ble-payload-v1:{:016x}", fnv1a64(&bytes))
}

fn ibeacon_identity(advertisement: &Advertisement) -> Option<Vec<u8>> {
    advertisement
        .manufacturer_data
        .iter()
        .find(|item| {
            item.company_id == 0x004c
                && item.data.len() >= 23
                && item.data[0] == 0x02
                && item.data[1] == 0x15
        })
        .map(|item| item.data[2..22].to_vec())
}

fn eddystone_uid(advertisement: &Advertisement) -> Option<Vec<u8>> {
    advertisement
        .service_data
        .iter()
        .find(|item| {
            normalized_uuid(&item.uuid).ends_with("feaa")
                && item.data.len() >= 18
                && item.data[0] == 0x00
        })
        .map(|item| item.data[2..18].to_vec())
}

fn identity(protocol: &str, bytes: &[u8], confidence: IdentityConfidence) -> Identity {
    Identity {
        key: format!("ble-id-v1:{:016x}", fnv1a64(bytes)),
        confidence,
        protocol: (protocol != "address").then(|| protocol.to_string()),
    }
}

fn normalized_uuid(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .flat_map(char::to_lowercase)
        .collect()
}

fn push_text(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
