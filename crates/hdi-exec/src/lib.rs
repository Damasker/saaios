//! Read-only HDI executor (ADR-426, sprint HDI-02).
//!
//! Reads a fixture root and writes evidence only under the campaign
//! directory. Probe templates are static reads. This crate does not spawn
//! a shell, write sysfs, or depend on the modem, shell, or taskd.

mod execute;
mod template;

pub use execute::{Evidence, ExecError, Executor, Fault, JournalEntry, JournalVerdict};
pub use template::{probe_templates, ProbeTemplate};
