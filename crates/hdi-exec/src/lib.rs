//! Read-only HDI executor (ADR-426, sprint HDI-02).
//!
//! Reads a fixture root and writes evidence only under the campaign
//! directory. `sysfs_read` uses a separate root: the fixture directory by
//! default, or a caller-supplied root that is `/` on a device and a
//! temporary directory on the host. Probe templates are static reads.
//! This crate does not spawn a shell, write sysfs, or depend on the modem,
//! shell, or taskd.

mod execute;
mod template;

pub use execute::{Evidence, ExecError, Executor, Fault, JournalEntry, JournalVerdict};
pub use template::{probe_templates, ProbeTemplate};
