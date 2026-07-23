use alloc::vec::Vec;

use radiochron::ble::{Advertisement, Collector};

/// Testable boundary over an ESP-IDF BLE scanner.
pub trait BleDriver {
    type Error;

    fn scan(&mut self, output: &mut Vec<Advertisement>) -> Result<(), Self::Error>;
}

/// RadioChron BLE collector owning an ESP-IDF scanner implementation.
pub struct EspIdfBleCollector<D> {
    driver: D,
}

impl<D> EspIdfBleCollector<D> {
    pub const fn new(driver: D) -> Self {
        Self { driver }
    }

    pub fn driver_mut(&mut self) -> &mut D {
        &mut self.driver
    }

    pub fn into_inner(self) -> D {
        self.driver
    }
}

impl<D: BleDriver> Collector for EspIdfBleCollector<D> {
    type Error = D::Error;

    fn scan(&mut self, output: &mut Vec<Advertisement>) -> Result<(), Self::Error> {
        self.driver.scan(output)
    }
}
