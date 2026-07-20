//! BeaconTrail MCP server.
//!
//! Speaks the Model Context Protocol over stdio. Register it with an MCP client
//! (Claude Code, Claude Desktop, Codex, …) by pointing the client at this
//! binary; no arguments are required.
//!
//! On the stdio transport stdout carries JSON-RPC frames and nothing else, so
//! all diagnostics must go to stderr.

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    beacontrail::mcp::serve_stdio()
}

#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    anyhow::bail!("BeaconTrail requires Windows (it talks to wlanapi.dll).")
}
