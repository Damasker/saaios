//! Host-safe OS gate for panther: cellular is unavailable; boot continues.
//! No modem endpoints, mounts, or SIT.

/// Stable reason: CP acquires only the foreign WCDMA camp and completes LTE
/// scans empty. Host registration/scan levers are exhausted (VERDICT 24–27).
pub const REASON_CP_RF_CELL_SELECTION_WALL: &str = "cp_rf_cell_selection_wall";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OsGate {
    pub continue_os: bool,
    pub cellular_service: CellularService,
    pub network_authority: NetworkAuthority,
    pub reason: &'static str,
    pub scan_axis: ScanAxis,
    pub host_registration_levers: HostLevers,
    pub bearer_verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellularService {
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAuthority {
    Wlan0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanAxis {
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLevers {
    Exhausted,
}

impl Default for OsGate {
    fn default() -> Self {
        Self {
            continue_os: true,
            cellular_service: CellularService::Unavailable,
            network_authority: NetworkAuthority::Wlan0,
            reason: REASON_CP_RF_CELL_SELECTION_WALL,
            scan_axis: ScanAxis::Closed,
            host_registration_levers: HostLevers::Exhausted,
            bearer_verified: false,
        }
    }
}

impl OsGate {
    pub fn lines(self) -> Vec<String> {
        vec![
            format!(
                "continue_os={}",
                if self.continue_os { "yes" } else { "no" }
            ),
            format!(
                "cellular_service={}",
                match self.cellular_service {
                    CellularService::Unavailable => "unavailable",
                }
            ),
            format!(
                "network_authority={}",
                match self.network_authority {
                    NetworkAuthority::Wlan0 => "wlan0",
                }
            ),
            format!("reason={}", self.reason),
            format!(
                "scan_axis={}",
                match self.scan_axis {
                    ScanAxis::Closed => "closed",
                }
            ),
            format!(
                "host_registration_levers={}",
                match self.host_registration_levers {
                    HostLevers::Exhausted => "exhausted",
                }
            ),
            format!(
                "bearer_verified={}",
                if self.bearer_verified { "yes" } else { "no" }
            ),
            "hardware_actions=none".to_string(),
            "doc=docs/os/targets/panther/MODEM-OS-CONTRACT.md".to_string(),
        ]
    }
}

/// Shell/PID1 must boot even when this binary is absent. Presence of
/// `continue_os=yes` never means “wait for modem”.
pub fn pid1_may_skip_modemd() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_continues_without_cellular() {
        let gate = OsGate::default();
        assert!(gate.continue_os);
        assert!(!gate.bearer_verified);
        assert!(pid1_may_skip_modemd());
        let blob = gate.lines().join("\n");
        assert!(blob.contains("continue_os=yes"));
        assert!(blob.contains("cellular_service=unavailable"));
        assert!(blob.contains("network_authority=wlan0"));
        assert!(blob.contains("scan_axis=closed"));
        assert!(blob.contains("host_registration_levers=exhausted"));
        assert!(blob.contains("hardware_actions=none"));
        assert!(!blob.contains("continue_os=no"));
    }
}
