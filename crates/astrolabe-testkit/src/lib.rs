//! Optional original-engine tooling. The converter never depends on this crate.
//! Process addresses and capture provenance are local evidence, not Rete data.
#[cfg(target_os = "linux")]
pub mod linux;
