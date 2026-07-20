//! The loop that writes the chronicle on Windows.
//!
//! Combines two sources into one stream of entries: polled association state
//! (through the pure [`ChangeDetector`]) and, when the `history` feature is
//! compiled in, new WLAN AutoConfig events — which is where reason codes live.
//!
//! The caller owns the loop. [`Recorder::step`] does one poll and returns how
//! many entries were written, so an IoT agent embeds it in its own scheduler,
//! and [`Recorder::run_for`] is the convenience wrapper for everyone else.

use std::time::Duration;

use super::{ChangeDetector, Entry, Observation, Sink};
use crate::wlan;

#[derive(Debug, Clone, Copy)]
pub struct RecorderOptions {
    /// Delay between polls in [`Recorder::run_for`].
    pub interval: Duration,
    /// Hysteresis for [`EntryKind::SignalShift`], in dB.
    pub signal_threshold_db: i32,
}

impl Default for RecorderOptions {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(5),
            signal_threshold_db: 8,
        }
    }
}

pub struct Recorder<S: Sink> {
    sink: S,
    detector: ChangeDetector,
    options: RecorderOptions,
    /// Only events newer than this reach the chronicle. Starts at construction
    /// time: the chronicle records what happens *while recording* — the past is
    /// already served by [`crate::events::recent`], and dumping it here would
    /// duplicate every event on every restart.
    #[cfg(feature = "history")]
    last_log_epoch: i64,
}

impl<S: Sink> Recorder<S> {
    pub fn new(sink: S, options: RecorderOptions) -> Self {
        Self {
            sink,
            detector: ChangeDetector::new(options.signal_threshold_db),
            options,
            #[cfg(feature = "history")]
            last_log_epoch: crate::time::now_epoch_seconds(),
        }
    }

    /// One poll: observe, detect, tail the log, write. Returns entries written.
    ///
    /// A failed status read is recorded as a disconnected observation rather
    /// than aborting the run — for a recorder, "could not see the radio" is
    /// itself a data point, and a transient WLAN-service hiccup must not end a
    /// week-long recording.
    pub fn step(&mut self) -> anyhow::Result<usize> {
        let observation = match wlan::wifi_status() {
            Ok(status) => status
                .into_iter()
                .find_map(|s| s.connection)
                .map(|c| Observation {
                    connected: true,
                    ssid: c.ssid,
                    bssid: c.bssid,
                    rssi_dbm: Some(c.rssi_dbm_estimate),
                })
                .unwrap_or_default(),
            Err(_) => Observation::default(),
        };

        #[cfg_attr(not(feature = "history"), allow(unused_mut))]
        let mut kinds = self.detector.observe(observation);

        #[cfg(feature = "history")]
        kinds.extend(self.tail_event_log());

        let written = kinds.len();
        for kind in kinds {
            self.sink.write(&Entry::now(kind))?;
        }
        if written > 0 {
            self.sink.flush()?;
        }

        Ok(written)
    }

    /// Poll on the configured interval until `duration` elapses.
    pub fn run_for(&mut self, duration: Duration) -> anyhow::Result<usize> {
        let started = std::time::Instant::now();
        let mut total = 0;

        loop {
            total += self.step()?;
            if started.elapsed() + self.options.interval > duration {
                return Ok(total);
            }
            std::thread::sleep(self.options.interval);
        }
    }

    /// Recover the sink (for tests and for callers that reuse it).
    pub fn into_sink(self) -> S {
        self.sink
    }

    /// New WLAN AutoConfig events since the last step, oldest first.
    ///
    /// A read failure yields nothing rather than an error: the log is the
    /// *secondary* source here, and an access-denied must not kill a recording
    /// that the polled source is still feeding.
    #[cfg(feature = "history")]
    fn tail_event_log(&mut self) -> Vec<super::EntryKind> {
        // Look back far enough to bridge slow polls; the epoch filter dedupes.
        let Ok(events) = crate::events::recent(64, Some(120)) else {
            return Vec::new();
        };

        let mut fresh: Vec<super::EntryKind> = events
            .iter()
            .filter(|e| e.epoch_seconds > self.last_log_epoch)
            .map(|e| super::EntryKind::LogEvent {
                event_id: e.event_id,
                meaning: e.meaning.to_string(),
                fields: e.data.clone(),
            })
            .collect();
        // `recent` is newest-first; a chronicle reads oldest-first.
        fresh.reverse();

        if let Some(newest) = events.iter().map(|e| e.epoch_seconds).max() {
            self.last_log_epoch = self.last_log_epoch.max(newest);
        }

        fresh
    }
}
