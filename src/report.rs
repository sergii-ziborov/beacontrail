//! Human-readable diagnostic report, exposed as an MCP resource.
//!
//! Resources are application-driven: the host decides whether to pull them into
//! context, with no tool call and no model decision. That makes this the
//! zero-friction path for "here is what my Wi-Fi looks like", while the tools
//! remain the path for model-initiated and parameterised queries.
//!
//! Deliberately serves the CACHED scan. A read must never trigger a four-second
//! scan: clients may read resources speculatively or on every turn, and a hidden
//! multi-second stall on a passive read is bad behaviour. Freshness is made
//! legible in the body instead.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::wlan::analyze::Analysis;
use crate::wlan::bss::BssEntry;
use crate::wlan::WifiStatus;

/// Current UTC time as RFC 3339, without pulling in a date library.
///
/// Uses Howard Hinnant's civil-from-days algorithm: the era arithmetic below is
/// exact for all dates we can encounter and is far cheaper than a dependency
/// whose Windows feature would drag the whole build-toolchain problem back in.
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days);

    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so leap days land at the end of the cycle.
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;

    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Render the report as Markdown.
pub fn markdown(status: &[WifiStatus], entries: &[BssEntry], analysis: &Analysis) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# Wi-Fi Diagnostic Report\n");
    let _ = writeln!(out, "Generated: {}", now_iso8601());
    let _ = writeln!(
        out,
        "Source: cached scan results — call the wifi_scan tool to force a refresh.\n"
    );

    let _ = writeln!(out, "## Adapters\n");
    for entry in status {
        let _ = writeln!(
            out,
            "- **{}** — {}",
            entry.interface.description, entry.interface.state
        );
    }

    match status.iter().find_map(|s| s.connection.as_ref()) {
        Some(connection) => {
            let _ = writeln!(out, "\n## Association\n");
            let _ = writeln!(
                out,
                "| SSID | BSSID | PHY | Quality | RSSI | Rx | Tx |\n|---|---|---|---|---|---|---|"
            );
            let _ = writeln!(
                out,
                "| {} | {} | {} | {}/100 | {} dBm | {} kbps | {} kbps |",
                connection.ssid.as_deref().unwrap_or("—"),
                connection.bssid.as_deref().unwrap_or("—"),
                connection.phy_type,
                connection.signal_quality,
                connection.rssi_dbm_estimate,
                connection.rx_rate_kbps,
                connection.tx_rate_kbps
            );
        }
        None => {
            let _ = writeln!(out, "\n## Association\n\nNot associated.");
        }
    }

    let _ = writeln!(out, "\n## Environment\n");
    let _ = writeln!(out, "{} BSS visible.\n", entries.len());
    if !analysis.bands.is_empty() {
        let _ = writeln!(
            out,
            "| Band | BSS | SSIDs | Channels | Strongest |\n|---|---|---|---|---|"
        );
        for band in &analysis.bands {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} dBm |",
                band.band,
                band.bss_count,
                band.distinct_ssids,
                band.distinct_channels,
                band.strongest_dbm.unwrap_or(0)
            );
        }
    }

    let _ = writeln!(out, "\n## Findings\n");
    if analysis.findings.is_empty() {
        let _ = writeln!(out, "None. Nothing in the environment looks wrong.");
    } else {
        for finding in &analysis.findings {
            let _ = writeln!(out, "### [{}] {}\n", finding.severity, finding.title);
            let _ = writeln!(out, "{}\n", finding.caveat);
        }
    }

    let _ = writeln!(out, "\n## Strongest BSS\n");
    let _ = writeln!(
        out,
        "| SSID | BSSID | Band | Ch | RSSI | RSN |\n|---|---|---|---|---|---|"
    );

    let mut sorted: Vec<&BssEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| -e.rssi_dbm);
    for entry in sorted.iter().take(15) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            entry.ssid.as_deref().unwrap_or("*hidden*"),
            entry.bssid,
            entry.band,
            entry
                .channel
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into()),
            entry.rssi_dbm,
            if entry.information_elements.has_rsn {
                "yes"
            } else {
                "no"
            }
        );
    }

    let _ = writeln!(
        out,
        "\n---\n\nThis report lists SSIDs and BSSIDs of nearby networks, including \
         neighbours'. A BSSID can be resolved to a street address through public \
         geolocation databases — treat it as location-identifying before sharing."
    );

    out
}

/// Machine-readable form of the same snapshot.
pub fn json(status: &[WifiStatus], entries: &[BssEntry], analysis: &Analysis) -> Value {
    json!({
        "generated": now_iso8601(),
        "source": "cached scan results",
        "adapters": status,
        "bss_count": entries.len(),
        "analysis": analysis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(1), (1970, 1, 2));
        // 2000-03-01: the leap-year boundary the era arithmetic exists to get right.
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn timestamp_is_rfc3339_shaped() {
        let stamp = now_iso8601();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'));
        assert_eq!(stamp.as_bytes()[4], b'-');
        assert_eq!(stamp.as_bytes()[10], b'T');
    }
}
