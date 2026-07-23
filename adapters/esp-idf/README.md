# radiochron-esp-idf

ESP-IDF Wi-Fi and optional NimBLE collector adapter for the `radiochron` Rust
diagnostics core. ESP dependencies stay here and never enter the core crate.

```toml
[dependencies]
radiochron = { version = "0.4", default-features = false, features = ["embedded"] }
radiochron-esp-idf = "0.2"
```

Wrap the `BlockingWifi<EspWifi>` already created by the application (`EspWifi`
itself is supported too):

```rust,ignore
use radiochron::embedded::Snapshot;
use radiochron_esp_idf::EspIdfCollector;

let mut collector = EspIdfCollector::new(wifi);
let mut snapshot = Snapshot::new();
snapshot.refresh(&mut collector)?;
let analysis = snapshot.analyze();
```

The adapter uses `is_connected`, `get_ap_info` and the blocking ESP-IDF scan.
It maps SSID, BSSID, RSSI, channel, PHY and the SDK-reported authentication
mode. ESP-IDF's high-level scan result does not carry raw beacon Information
Elements, so entries set `ie_data_complete = false`; RadioChron will use the
reported authentication mode but will not invent RSN/PMF details.

Scanning wakes the radio and can interrupt power-saving schedules. Firmware
owns the cadence and should call `Snapshot::refresh` according to its energy
budget.

For the embedded chronicle, subscribe to ESP-IDF's `WifiEvent` on the system
event loop and pass `radiochron_esp_idf::disconnect_reason(&event)` to
`Chronicle::observe_status_with_reason`. This preserves the SDK/IEEE reason code
without coupling the RadioChron core to ESP-IDF's event-loop implementation.

## BLE with NimBLE

BLE is opt-in, so existing Wi-Fi firmware does not compile or initialize a
Bluetooth stack:

```toml
radiochron = { version = "0.4", default-features = false, features = ["embedded", "ble"] }
radiochron-esp-idf = { version = "0.2", features = ["nimble"] }
```

Enable NimBLE in `sdkconfig.defaults`:

```text
CONFIG_BT_ENABLED=y
CONFIG_BT_BLE_ENABLED=y
CONFIG_BT_BLUEDROID_ENABLED=n
CONFIG_BT_NIMBLE_ENABLED=y
```

Then scan into the portable RadioChron model:

```rust,ignore
use radiochron::ble::Snapshot;
use radiochron_esp_idf::{EspIdfBleCollector, NimbleBleDriver};

let driver = NimbleBleDriver::take(5_000);
let mut collector = EspIdfBleCollector::new(driver);
let mut snapshot = Snapshot::new();
snapshot.refresh(&mut collector)?;
```

`ble` exposes only the host-testable adapter contract. `nimble` additionally
activates `esp32-nimble` on ESP-IDF targets and performs a real active scan.
Firmware still owns scan cadence, energy budget, clock, retention and export.
