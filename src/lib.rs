//! BeaconTrail — pure-Rust Windows Wi-Fi collectors, exposed over MCP.
//!
//! The engine reaches the native Windows WLAN stack through hand-written FFI
//! (see [`wlan::sys`]) rather than by spawning `netsh` / PowerShell or compiling
//! embedded C# at runtime, which is how the original TypeScript/Electron
//! implementation worked. Nothing here shells out, and nothing leaves the
//! machine.
//!
//! - [`wlan`] — interface state, current connection, BSS list, 802.11 IE parsing
//! - [`mcp`] — the read-only Model Context Protocol tool surface

#[cfg(windows)]
pub mod mcp;

#[cfg(windows)]
pub mod report;

#[cfg(windows)]
pub mod wlan;
