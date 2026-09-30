//! MODEM-06 soft-lock detection (pure; no device I/O).
//!
//! Stock ONLINE often reports app_state=PIN while Present (inferred from
//! app_state / last STATUS→SET_APP decision — **not** a wire `+0xBF6` peek)
//! is outside {1,2,3}. START_NETWORK then never runs.
//!
//! Two live shapes share the same chicken-egg:
//! - pin1=DISABLED(3): classic soft-lock
//! - pin1=ENABLED_VERIFIED(2): Pin1Verified OK remotely, app still PIN
//!
//! HotSwap ABSENT→PRESENT (2026-09-30 live) falsified tray reseat as a
//! READY/bearer path on EU No-CDMA: VerifyPin OK, app stayed PIN,
//! present_infer=notin_1_2_3. Remaining options are banned (EFS TCS /
//! unsigned MAIN / stock rild). Chase remains observability + post-READY
//! plumbing only.

/// Factory SIT app_state: PIN required.
pub const APP_STATE_PIN: u8 = 2;
/// Factory pin1: ENABLED_VERIFIED (Pin1Verified set; app may still be PIN).
pub const PIN1_ENABLED_VERIFIED: u8 = 2;
/// Factory pin1: DISABLED (no PIN entry path).
pub const PIN1_DISABLED: u8 = 3;

/// Present enum values that can leave the soft-lock (STATUS/READY paths).
pub const PRESENT_STATUS: u8 = 1;
pub const PRESENT_READY: u8 = 2;
pub const PRESENT_DETECTED: u8 = 3;

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
    /// Present byte when known from RO; None means inferred-not-in-{1,2,3}
    /// (tray-watch `present_infer=notin_1_2_3`).
    pub present: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftLockVerdict {
    /// PIN + Present ∉ {1,2,3} with pin1 DISABLED or ENABLED_VERIFIED.
    Modem06SoftLock,
    NotSoftLock,
}

impl SoftLockSnapshot {
    pub fn present_in_unlock_set(present: u8) -> bool {
        matches!(
            present,
            PRESENT_STATUS | PRESENT_READY | PRESENT_DETECTED
        )
    }

    /// pin1 shapes that still leave START_NETWORK blocked when app=PIN.
    pub fn pin1_in_soft_lock_set(pin1: u8) -> bool {
        matches!(pin1, PIN1_DISABLED | PIN1_ENABLED_VERIFIED)
    }

    pub fn verdict(self) -> SoftLockVerdict {
        if self.app_state != APP_STATE_PIN || !Self::pin1_in_soft_lock_set(self.pin1) {
            return SoftLockVerdict::NotSoftLock;
        }
        match self.present {
            Some(p) if Self::present_in_unlock_set(p) => SoftLockVerdict::NotSoftLock,
            // None = operator/tray reported notin_1_2_3; Some outside set = same.
            _ => SoftLockVerdict::Modem06SoftLock,
        }
    }

    pub fn is_modem06(self) -> bool {
        self.verdict() == SoftLockVerdict::Modem06SoftLock
    }
}

/// Parse `present_infer=notin_1_2_3` / `present_infer=1` style tokens (no secrets).
pub fn parse_present_infer(token: &str) -> Option<Option<u8>> {
    let t = token.trim();
    if t.eq_ignore_ascii_case("notin_1_2_3") || t.eq_ignore_ascii_case("notin") {
        return Some(None);
    }
    let v: u8 = t.parse().ok()?;
    Some(Some(v))
}

/// Scan tray-watch / chase log lines for MODEM-06 fields without ICCID/IMSI.
pub fn detect_from_status_text(text: &str) -> Option<SoftLockSnapshot> {
    let mut app: Option<u8> = None;
    let mut pin1: Option<u8> = None;
    let mut present: Option<Option<u8>> = None;

    for raw in text.split_whitespace() {
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
        } else if let Some(rest) = raw.strip_prefix("present_infer=") {
            present = parse_present_infer(rest);
        }
    }

    Some(SoftLockSnapshot {
        app_state: app?,
        pin1: pin1?,
        present: present.unwrap_or(None),
    })
}

pub fn advice_lines(snapshot: SoftLockSnapshot) -> Vec<String> {
    match snapshot.verdict() {
        SoftLockVerdict::Modem06SoftLock => {
            let shape = if snapshot.pin1 == PIN1_ENABLED_VERIFIED {
                "pin1_verified_chicken_egg"
            } else {
                "pin1_disabled"
            };
            vec![
                "modem06_soft_lock=yes".into(),
                format!("soft_lock_shape={shape}"),
                format!("action={TRAY_BEARER_CHASE_CMD}"),
                format!("on_device={TRAY_BEARER_CHASE_ON_DEVICE}"),
                "note=reseat_sim_tray_during_watch; do_not_duplicate_concurrent_watch".into(),
                "fn_a_ratmap=blocked_EU_NoCDMA_NV_only_no_signed_SIT_no_CDMA_preferred_try".into(),
                "cp_start_network=hard_reject_err2_while_pin".into(),
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
        assert!(advice.iter().any(|l| l.contains("pin1_verified_chicken_egg")));
    }

    #[test]
    fn unlock_present_clears_soft_lock() {
        let s = SoftLockSnapshot {
            app_state: 2,
            pin1: 3,
            present: Some(PRESENT_READY),
        };
        assert!(!s.is_modem06());
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
        assert!(advice.iter().any(|l| l.contains("fn_a_ratmap=blocked")));
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
}
