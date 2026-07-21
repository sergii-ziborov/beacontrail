# radiochron

**[radiochron.com](https://radiochron.com)** · the chronicle of your radio.

Wi-Fi diagnostics as an MCP server: connection-history **verdicts** (reconnect
loops, an AP failing key exchange, credential mismatch), findings instead of
data dumps, a change-only chronicle recorder, and native WLAN collectors —
pure Rust, shipped here as a prebuilt binary. No Rust toolchain, no git, no
build step.

## Install

```sh
claude mcp add radiochron -- npx -y radiochron
```

Or in any MCP client config:

```json
{ "mcpServers": { "radiochron": { "command": "npx", "args": ["-y", "radiochron"] } } }
```

Ten typed tools: the six Wi-Fi diagnostics plus `chronicle_start`,
`chronicle_stop`, `chronicle_status` and `chronicle_recent`. Scans update the
driver cache and the opt-in chronicle writes a rotating local JSONL file;
nothing reads saved passwords or leaves the machine.

**Windows-only today** — the engine talks to `wlanapi.dll` and the WLAN event
log directly. Linux (nl80211) and macOS (CoreWLAN) are on the
[roadmap](https://radiochron.com/#roadmap), as are napi-rs bindings for
importing the library from Node.js.

MIT OR Apache-2.0, at your option · [source](https://github.com/sergii-ziborov/radiochron)
