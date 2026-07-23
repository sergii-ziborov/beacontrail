use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use esp_idf_svc::hal::task::block_on;
use esp32_nimble::enums::AdvType;
use esp32_nimble::{
    BLEAddress, BLEAddressType, BLEAdvertisedData, BLEAdvertisedDevice, BLEDevice, BLEError,
    BLEScan,
};

use radiochron::ble::{
    AddressType, Advertisement, ManufacturerData, ServiceData,
};

use super::BleDriver;

/// Blocking RadioChron adapter over ESP-IDF's NimBLE scan task.
pub struct NimbleBleDriver {
    device: &'static BLEDevice,
    scan: BLEScan,
    duration_ms: i32,
}

impl NimbleBleDriver {
    /// Initialize NimBLE and take its process-wide device singleton.
    pub fn take(duration_ms: i32) -> Self {
        let device: &'static BLEDevice = BLEDevice::take();
        let mut scan = BLEScan::new();
        scan.active_scan(true)
            .filter_duplicates(false)
            .interval(100)
            .window(99);
        Self {
            device,
            scan,
            duration_ms: duration_ms.max(1),
        }
    }

    pub fn set_duration_ms(&mut self, duration_ms: i32) {
        self.duration_ms = duration_ms.max(1);
    }
}

impl BleDriver for NimbleBleDriver {
    type Error = BLEError;

    fn scan(&mut self, output: &mut Vec<Advertisement>) -> Result<(), Self::Error> {
        block_on(self.scan.start(
            self.device,
            self.duration_ms,
            |device, data| {
                output.push(map_advertisement(device, &data));
                None::<()>
            },
        ))
        .map(|_| ())
    }
}

fn map_advertisement(
    device: &BLEAdvertisedDevice,
    data: &BLEAdvertisedData<&[u8]>,
) -> Advertisement {
    let address = device.addr();
    let manufacturer_data = data
        .manufacture_data()
        .map(|item| {
            vec![ManufacturerData {
                company_id: item.company_identifier,
                data: item.payload.to_vec(),
            }]
        })
        .unwrap_or_default();
    let service_data = data
        .service_data()
        .map(|item| {
            vec![ServiceData {
                uuid: item.uuid.to_string(),
                data: item.service_data.to_vec(),
            }]
        })
        .unwrap_or_default();

    Advertisement {
        address: address.to_string(),
        address_type: address_type(&address),
        local_name: data
            .name()
            .map(|name| String::from_utf8_lossy(name.as_ref()).into_owned()),
        rssi_dbm: i16::from(device.rssi()),
        tx_power_dbm: data.tx_power().map(|value| i16::from(value as i8)),
        connectable: Some(matches!(
            device.adv_type(),
            AdvType::Ind | AdvType::DirectInd
        )),
        service_uuids: data
            .service_uuids()
            .map(|uuid| uuid.to_string())
            .collect(),
        manufacturer_data,
        service_data,
        protocol_identity: None,
    }
}

fn address_type(address: &BLEAddress) -> AddressType {
    match address.addr_type() {
        BLEAddressType::Public | BLEAddressType::PublicID => AddressType::Public,
        BLEAddressType::Random | BLEAddressType::RandomID => {
            match address.as_be_bytes()[0] >> 6 {
                0b11 => AddressType::RandomStatic,
                0b01 => AddressType::ResolvablePrivate,
                0b00 => AddressType::NonResolvablePrivate,
                _ => AddressType::Unknown,
            }
        }
    }
}
