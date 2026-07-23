use alloc::vec::Vec;

use crate::ble::{Advertisement, Finding, Observation, ObservationResult, SensorContext, Tracker};

use super::{Chronicle, Clock, EventKind, Sink};

impl<S: Sink, C: Clock> Chronicle<S, C> {
    /// Add one BLE advertisement to history and record every finding produced
    /// from it. Time comes from the same injected clock as Wi-Fi events.
    pub fn observe_ble(
        &mut self,
        context: SensorContext,
        advertisement: Advertisement,
        tracker: &mut Tracker,
    ) -> Result<ObservationResult, S::Error> {
        let reading = self.clock_mut().now();
        let sensor_id = context.sensor_id.clone();
        let result = tracker.observe(Observation {
            monotonic_ms: reading.monotonic_ms,
            unix_epoch_ms: reading.unix_epoch_ms,
            context,
            advertisement: advertisement.clone(),
        });
        self.record(
            Some(sensor_id.clone()),
            EventKind::BleObservation {
                sensor_id,
                identity: result.identity.clone(),
                payload_hash: result.payload_hash.clone(),
                rssi_dbm: advertisement.rssi_dbm,
            },
        )?;
        for finding in &result.findings {
            self.record(
                None,
                EventKind::BleFinding {
                    finding: finding.clone(),
                },
            )?;
        }
        Ok(result)
    }

    /// Evaluate time-based BLE rules such as an expected beacon disappearing.
    pub fn evaluate_ble(&mut self, tracker: &mut Tracker) -> Result<Vec<Finding>, S::Error> {
        let now_ms = self.clock_mut().now().monotonic_ms;
        let findings = tracker.evaluate(now_ms);
        for finding in &findings {
            self.record(
                None,
                EventKind::BleFinding {
                    finding: finding.clone(),
                },
            )?;
        }
        Ok(findings)
    }
}
