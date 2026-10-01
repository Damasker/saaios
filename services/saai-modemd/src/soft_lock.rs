//! MODEM-06 soft-lock detection (pure; no device I/O).
//!
//! Stock ONLINE often reports app_state=PIN. The SIT status reply carries the
//! published app state, but not the CP's current Present byte at `+0xBF6`.
//! START_NETWORK rejects the published PIN state; the missing transition to
//! READY is still under investigation.
//!
//! Observed PIN forms include pin1=DISABLED(3), ENABLED_VERIFIED(2), and
//! NOT_VERIFIED(1). The last form has a reproduced explicit VerifyPin path.
//!
//! A concurrent diagnostic run twice reached READY with VerifyPin A+AID
//! while pin1=1, without CardPower. It also held the SIT channel during an
//! RFS test, so that test cannot establish self-init or RFS causality.
//! Catalog `0x2f50` is an internal CP identifier, not an evidenced
//! AP-sendable frame.
//! Signed CPIF caps (AP part0=3 / CP part0=7) were exercised at INIT_START;
//! they are not a READY lever. Chase remains observability + post-READY
//! plumbing only.

/// Factory SIT app_state: PIN required.
pub const APP_STATE_PIN: u8 = 2;
/// Factory pin1: ENABLED_VERIFIED (Pin1Verified set; app may still be PIN).
pub const PIN1_ENABLED_VERIFIED: u8 = 2;
/// Factory pin1: DISABLED (no PIN entry path).
pub const PIN1_DISABLED: u8 = 3;

/// Signed CPIF INIT_START negotiate (exercised live; not a READY lever).
pub const CPIF_CAPS_NOTE: &str = "AP_part0=3(PKTPROC_UL|CH_EXT) CP_part0=7(+36BIT)";

/// The observed blocker is the published app state, not an inferred Present.
pub const BLOCKER_APP_PIN: &str = "cp_app_state_pin_blocks_start_network";

/// One-command operator chase after ONLINE (no secrets).
pub const TRAY_BEARER_CHASE_CMD: &str =
    "PERSIST=1 WATCH_ROUNDS=12 OUT=/data/saaios/var/tray-bearer.log \
sh os/targets/panther/diagnostics/tray-bearer-chase.sh";

/// On-device persistent watch (deployed to /data/saaios/bin; flock single-instance).
pub const TRAY_BEARER_CHASE_ON_DEVICE: &str =
    "PERSIST=1 WATCH_ROUNDS=12 nohup sh /data/saaios/bin/tray-bearer-chase.sh \
>/data/saaios/var/tray-bearer.nohup 2>&1 &";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoftLockSnapshot {
    pub app_state: u8,
    pub pin1: u8,
    /// Optional independently observed Present byte. SIT status text cannot
    /// supply it; None means unknown, never "not in {1,2,3}".
    pub present: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftLockVerdict {
    /// Published PIN blocks START_NETWORK regardless of PIN1 state.
    Modem06SoftLock,
    NotSoftLock,
}

impl SoftLockSnapshot {
    pub fn verdict(self) -> SoftLockVerdict {
        if self.app_state == APP_STATE_PIN {
            SoftLockVerdict::Modem06SoftLock
        } else {
            SoftLockVerdict::NotSoftLock
        }
    }

    pub fn is_modem06(self) -> bool {
        self.verdict() == SoftLockVerdict::Modem06SoftLock
    }
}

/// Parse one status line. App and PIN must come from the same observation.
fn parse_status_line(line: &str) -> Option<SoftLockSnapshot> {
    let mut app: Option<u8> = None;
    let mut pin1: Option<u8> = None;

    for raw in line.split_whitespace() {
        if let Some(rest) = raw.strip_prefix("app=") {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = num.parse() {
                app = Some(v);
            }
        } else if let Some(rest) = raw.strip_prefix("app0_state_raw=") {
            if let Ok(v) = rest.parse() {
                app = Some(v);
            }
        } else if let Some(rest) = raw.strip_prefix("app_raw=") {
            if let Ok(v) = rest.parse() {
                app = Some(v);
            }
        } else if let Some(rest) = raw.strip_prefix("pin1=") {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = num.parse() {
                pin1 = Some(v);
            }
        } else if let Some(rest) = raw.strip_prefix("pin1_state_raw=") {
            if let Ok(v) = rest.parse() {
                pin1 = Some(v);
            }
        }
    }

    Some(SoftLockSnapshot {
        app_state: app?,
        pin1: pin1?,
        present: None,
    })
}

/// Read the most recent usable line; a later failed query invalidates older
/// snapshots in a watch log. Never combine fields from different samples.
pub fn detect_from_status_text(text: &str) -> Option<SoftLockSnapshot> {
    for line in text.lines().rev() {
        if line.contains("tag=query_inconclusive") {
            return None;
        }
        if let Some(token) = line
            .split_whitespace()
            .find_map(|word| word.strip_prefix("query_rc="))
        {
            if token != "0" {
                return None;
            }
        }
        if let Some(snapshot) = parse_status_line(line) {
            return Some(snapshot);
        }
    }
    None
}

pub fn advice_lines(snapshot: SoftLockSnapshot) -> Vec<String> {
    match snapshot.verdict() {
        SoftLockVerdict::Modem06SoftLock => {
            let shape = match snapshot.pin1 {
                PIN1_ENABLED_VERIFIED => "pin1_verified_app_still_pin",
                PIN1_DISABLED => "pin1_disabled_app_still_pin",
                0 | 1 => "pin1_unverified_explicit_verify_path",
                _ => "pin1_unknown_app_pin",
            };
            vec![
                "modem06_soft_lock=yes".into(),
                format!("soft_lock_shape={shape}"),
                "cpif_caps_exercised=yes".into(),
                format!("cpif_caps_note={CPIF_CAPS_NOTE}"),
                format!("blocker={BLOCKER_APP_PIN}"),
                "present_status=unknown_not_on_sit_wire".into(),
                "panther_observed=verify_pin_a_aid_without_cardpower_ready_twice".into(),
                "rfs_causality=unproven_concurrent_verify_pin_sit_owner".into(),
                "next=explicit_verify_pin_only_if_pin1_unverified_then_fresh_status".into(),
                format!("observability={TRAY_BEARER_CHASE_CMD}"),
                format!("on_device_watch={TRAY_BEARER_CHASE_ON_DEVICE}"),
                "cp_start_network=blocked_while_app_pin".into(),
                "goal=incomplete_until_rmnet_ipv4".into(),
            ]
        }
        SoftLockVerdict::NotSoftLock => vec![
            "modem06_soft_lock=no".into(),
            "action=none".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_live_soft_lock_snapshot() {
        let s = SoftLockSnapshot {
            app_state: 2,
            pin1: 3,
            present: None,
        };
        assert!(s.is_modem06());
    }

    #[test]
    fn detects_pin1_verified_chicken_egg() {
        let s = SoftLockSnapshot {
            app_state: 2,
            pin1: PIN1_ENABLED_VERIFIED,
            present: None,
        };
        assert!(s.is_modem06());
        let advice = advice_lines(s);
        assert!(advice.iter().any(|l| l.contains("pin1_verified_app_still_pin")));
    }

    #[test]
    fn present_value_does_not_override_published_pin_state() {
        let s = SoftLockSnapshot {
            app_state: 2,
            pin1: 3,
            present: Some(2),
        };
        assert!(s.is_modem06());
    }

    #[test]
    fn parses_tray_transition_line() {
        let line = "TRANSITION card=1 apps=1 app=2(PIN) pin1=3 present_infer=notin_1_2_3";
        let s = detect_from_status_text(line).unwrap();
        assert_eq!(s.app_state, 2);
        assert_eq!(s.pin1, 3);
        assert_eq!(s.present, None);
        assert!(s.is_modem06());
        let advice = advice_lines(s);
        assert!(advice[0].contains("yes"));
        assert!(advice.iter().any(|l| l.contains("tray-bearer-chase")));
        assert!(advice.iter().any(|l| l.contains("present_status=unknown")));
        assert!(advice.iter().any(|l| l.contains("cpif_caps_exercised=yes")));
        assert!(advice.iter().any(|l| l.contains("cpif_caps_note=")));
        assert!(advice.iter().any(|l| l.contains(BLOCKER_APP_PIN)));
        assert!(advice.iter().any(|l| l.contains("verify_pin_a_aid_without_cardpower_ready_twice")));
        assert!(advice.iter().any(|l| l.contains("rfs_causality=unproven")));
    }

    #[test]
    fn pin1_verified_reports_observed_blocker() {
        let s = SoftLockSnapshot {
            app_state: APP_STATE_PIN,
            pin1: PIN1_ENABLED_VERIFIED,
            present: None,
        };
        let advice = advice_lines(s);
        assert!(advice.iter().any(|l| l.contains("pin1_verified_app_still_pin")));
        assert!(advice.iter().any(|l| l == "cpif_caps_exercised=yes"));
        assert!(advice.iter().any(|l| l.contains(CPIF_CAPS_NOTE)));
        assert!(advice.iter().any(|l| l.contains(BLOCKER_APP_PIN)));
        assert!(!advice.iter().any(|l| l.contains("oem-ipc-inject")));
    }

    #[test]
    fn parses_chase_sim_snapshot_pin1_verified() {
        let line = "SIM snapshot app=2 pin1=2 remain=3 card=1 present_infer=notin_1_2_3";
        let s = detect_from_status_text(line).unwrap();
        assert_eq!(s.app_state, 2);
        assert_eq!(s.pin1, 2);
        assert!(s.is_modem06());
    }

    #[test]
    fn ready_app_is_not_soft_lock() {
        let line = "app=5(READY) pin1=1 present_infer=2";
        let s = detect_from_status_text(line).unwrap();
        assert!(!s.is_modem06());
    }

    #[test]
    fn app_pin_with_unverified_pin1_is_still_a_network_gate() {
        let s = SoftLockSnapshot {
            app_state: APP_STATE_PIN,
            pin1: 1,
            present: None,
        };
        assert!(s.is_modem06());
        assert!(advice_lines(s)
            .iter()
            .any(|line| line.contains("pin1_unverified_explicit_verify_path")));
    }

    #[test]
    fn latest_failed_query_does_not_reuse_stale_pin_snapshot() {
        let log = "SIM snapshot app=2 pin1=1 card=1 query_rc=0\nSIM snapshot app=? pin1=? card=? query_rc=1";
        assert!(detect_from_status_text(log).is_none());
    }

    #[test]
    fn does_not_mix_app_and_pin_from_different_lines() {
        assert!(detect_from_status_text("app=2\npin1=2").is_none());
    }

    #[test]
    fn latest_ready_snapshot_wins_after_pin() {
        let log = "SIM snapshot app=2 pin1=1 card=1 query_rc=0\nSIM snapshot app=5 pin1=2 card=1 query_rc=0";
        assert_eq!(detect_from_status_text(log).unwrap().app_state, 5);
    }
}
