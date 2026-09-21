//! Optional original-engine tooling. The converter never depends on this crate.
//! Process addresses and capture provenance are local evidence, not Rete data.
pub mod comparison;
#[cfg(target_os = "linux")]
pub mod config;
#[cfg(target_os = "linux")]
pub mod desktop;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub mod instrumentation;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub mod original;
pub mod replay;
