# BeaconTrail

Local-first Windows Wi-Fi diagnostics as a **Model Context Protocol (MCP)
server**. Pure Rust — no PowerShell, no `netsh` text scraping, no embedded C#,
no .NET, no Node.

Ask an AI assistant *"why is my Wi-Fi flaky?"* and let it read the actual radio
environment: real dBm, channel frequencies, and 802.11 capability flags parsed
straight from beacon frames.

> Early release. The collectors below work and are verified on real hardware;
> history, run comparison and the event timeline are still being ported.

## Why

Intermittent Wi-Fi failures are hard to explain once the connection recovers.
BeaconTrail exposes what Windows already knows — interface state, nearby BSSIDs
with real signal strength, security posture — through a protocol an assistant
can drive directly. No screenshots, no copy-pasted `netsh` output.

Wi-Fi APs broadcast **beacon** frames; BeaconTrail keeps the **trail**.

## The pure-Rust thesis

The predecessor (an Electron/TypeScript app) could not reach `wlanapi.dll` from
Node, so it compiled embedded C# at runtime via PowerShell `Add-Type` for every
scan. Each data path therefore depended on `powershell.exe` plus the .NET CSC
compiler, paid a cold-compile cost, parsed locale-dependent English text, and
tripped AV/WDAC on exactly the managed corporate machines it targeted.

BeaconTrail calls the same Win32 APIs through hand-written FFI. It does not
depend on the `windows` crate either: seven `wlanapi.dll` entry points and a
handful of `#[repr(C)]` structs are declared directly, and the DLL is resolved
at run time via `LoadLibraryW`. No import library, no `raw-dylib`, no `dlltool`,
no Visual C++ build tools.

| Data source | Native API | Replaces |
|---|---|---|
| Interface + current connection | `WlanQueryInterface` | `netsh wlan show interfaces` |
| Nearby BSS list, dBm, IEs | `WlanGetNetworkBssList` | embedded C# `Add-Type` |
| Scan trigger | `WlanScan` | embedded C# `Add-Type` |

The MCP layer is hand-written too: the stdio transport is newline-delimited
JSON-RPC 2.0, so an SDK that pulls an async runtime, a schema generator and a
mandatory `chrono` (whose `clock` feature drags in `windows-link`/`raw-dylib`)
would have reintroduced the exact build requirement this project avoids.

**Total dependency count: three** — `serde`, `serde_json`, `anyhow`.
Release binary: **~310 KB**. Runtime requirements on the target machine: none.

## Build

Needs nothing but [rustup](https://rustup.rs). No Visual C++ build tools, no
Windows SDK, no mingw, no administrator rights.

The GNU toolchain is self-sufficient — it ships its own linker — so on a machine
without Visual Studio, select it once:

```powershell
rustup default stable-x86_64-pc-windows-gnu
```

Then:

```powershell
cargo test                  # unit tests
cargo build --release       # target\release\beacontrail.exe
cargo run --example probe   # human-readable dump against the real adapter
```

The MSVC toolchain works too if you already have the Visual C++ build tools; CI
builds on GNU so that an MSVC-only assumption cannot creep in unnoticed.

## Use it with an MCP client

Register the binary. For Claude Code:

```powershell
claude mcp add beacontrail -- "C:\path\to\beacontrail.exe"
```

Or add it to a client config directly:

```json
{
  "mcpServers": {
    "beacontrail": {
      "command": "C:\\path\\to\\beacontrail.exe"
    }
  }
}
```

No arguments, no configuration, no environment variables.

## Tools

| Tool | Arguments | Returns |
|---|---|---|
| `wifi_status` | — | Every WLAN interface, its state, and for the associated one: SSID, BSSID, PHY type (`ht`/`vht`/`he`/`eht`), signal quality, estimated RSSI in dBm, rx/tx rates |
| `wifi_networks` | `refresh_scan?: boolean` | Nearby BSS entries: SSID, BSSID, real RSSI in dBm, channel center frequency, rates, and capability flags parsed from beacon IEs (RSN/WPA/HT/VHT/HE/EHT, vendor OUIs) |
| `wifi_scan` | — | Triggers a driver scan on each interface; returns how many accepted |

All three are read-only.

## Deliberately not exposed

The parent project grew collectors that are unsafe to hand to an autonomous
model. They are not part of this server's tool surface, and calling them returns
`-32601 unknown tool`:

- **plaintext saved Wi-Fi keys** — a model must not be able to read and leak credentials
- **adapter MAC change / adapter restart / computer rename** — privileged, disruptive, can drop the operator off the network
- **active LAN sweeps** — emits probe traffic, trips IDS on managed segments
- **external AI-review shell-out** — arbitrary process execution and off-box data flow

## Verified

Measured on an Intel Wi-Fi 6E AX211 in a dense office environment:

- 20/20 unit tests green, including C-ABI struct layout assertions
- `wifi_status` — connected, `phy=he`, −58 dBm, 649/432 Mbps
- `wifi_networks` — **58 BSS** across 2.4 GHz (15), 5 GHz (23) and 6 GHz (20);
  RSSI −91..−54 dBm; 57 of 58 RSN-protected; IE blobs 100–384 bytes
- Full MCP session (`initialize` → `tools/list` → three `tools/call` → `ping`)
  completes in ~4 s, essentially all of it the deliberate post-scan settle

A useful correctness signal: 6 GHz APs report `RSN`+`HE` with **no** HT/VHT
elements, which is exactly what the spec requires — a naive parser would not
produce that pattern.

## Roadmap

- WLAN AutoConfig event timeline via `wevtapi` (reconnect-loop detection)
- Baseline runs, run comparison and evidence reports over SQLite
- IP configuration via `GetAdaptersAddresses`

## Safety and privacy

SSIDs, BSSIDs, MAC addresses and event logs are sensitive. BeaconTrail is
local-first, has no telemetry, and transmits nothing off the machine. Only run
scans against networks you own or are authorized to test. This is not a packet
sniffer, a geolocation system, or offensive Wi-Fi tooling.

## License

[MIT](LICENSE)
