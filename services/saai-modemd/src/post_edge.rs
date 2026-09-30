//! Post-EDGE bearer pipeline (pure; no device I/O).
//!
//! RE (MAIN B `449eeab3…`, panther EU): READY(#5) is sole `SET_APP` at
//! STATUS after `+0xBF6` Present copy CMP#2. Present=2 only via FN_A
//! (CDMA L1 TIMING_LATCH/MEAS arg==3); FN_B unreachable. Image has
//! `No CDMA in SupportedRatMap` (NV/TCS on QM_MM_INIT) — FN_A never fires.
//! Preferred `0x070a` (11/12 live; CDMA enums exist) does **not** mutate
//! RatMap; do not cargo-cult CDMA preferred. PresentObj `#636c` is CP heap
//! (AP ATU/SHMEM unreachable). No signed SIT forces Present/+0xBF6=2.
//! HotSwap INSERT ≠ Present=2 writer (DBT strings only; SET#1 callers skip
//! FN_A). getobj`#0x10` @SET#1 = L1LC timer object, not Present — no poke.
//! Physical HotSwap ABSENT→PRESENT is a real EDGE but **live-falsified** as a
//! READY/bearer path on EU No-CDMA (2026-09-30): VerifyPin OK, app stayed PIN,
//! present_infer=notin (validated as STATUS decision, not +0xBF6 peek).
//!
//! SetupDataCall (`0x0600`): sit-stream simple builder length **246**
//! (`0xf6`); extended TD overload length **983** (`0x3d7`). Gated by
//! `isValidPdpApn` (non-NULL PdpContext + field). APN copied to packet
//! offset 16 (max 100). Never invent an APN string — resolve from
//! operator config (`/data/saaios/etc/apn`) or explicit CLI.
//!
//! After EDGE opens app∈{1,4,5}, this plan is the automatic path to
//! `rmnet` IPv4 / bidirectional rx+tx. START_NETWORK allows only {1,4,5}.

use crate::soft_lock::{
    SoftLockSnapshot, APP_STATE_PIN, PIN1_DISABLED, PIN1_ENABLED_VERIFIED,
};

/// DETECTED — START_NETWORK accepts.
pub const APP_DETECTED: u8 = 1;
/// SUBSCRIPTION_PERSO — START_NETWORK accepts.
pub const APP_SUBSCRIPTION_PERSO: u8 = 4;
/// READY — START_NETWORK accepts.
pub const APP_READY: u8 = 5;

/// Factory preferred-network enum for LTE_ONLY (live GET=11).
pub const PREFERRED_LTE_ONLY: u8 = 11;

/// Signed CPIF capability note (INIT_START negotiate; not a READY lever).
pub const CPIF_CAPS_NOTE: &str = "AP_part0=3(PKTPROC_UL|CH_EXT) CP_part0=7(+36BIT)";

/// Evidenced simple `BuildSetupDataCall` frame length (sit-stream `0x78c80`).
pub const SETUP_DATA_CALL_LEN_SIMPLE: u16 = 246;
/// Evidenced SIT id for SetupDataCall.
pub const SETUP_DATA_CALL_ID: u16 = 0x0600;
/// Evidenced empty GET `BuildGetDataCallList`.
pub const GET_DATA_CALL_LIST_ID: u16 = 0x0602;
/// Factory RO default when DB has no protocol: IPV4V6.
pub const SETUP_PROTO_IPV4V6: u8 = 3;
/// On-device APN path (operator-supplied; never invent carrier string).
pub const APN_CONFIG_PATH: &str = "/data/saaios/etc/apn";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostEdgeStep {
    /// CardPower+VerifyPin A+AID when pin1∈{0,1} and remain>1.
    VerifyPinIfNeeded,
    /// Abort chase if app still outside {1,4,5}.
    GateStartNetwork,
    /// Radio ON (no empty VerifyPin cycle).
    RadioOn,
    /// Preferred = LTE_ONLY (11).
    LteOnly,
    /// BuildSetNetworkSelectionAuto `0x0704`.
    NetworkSelectionAuto,
    /// BuildAllowData `0x0710` allow=1.
    AllowData,
    /// BuildGetPsService `0x0711` (signed empty GET; err0 seen live).
    GetPsService,
    /// Poll data-registration `0x0701` until searching/registered.
    PollDataRegistration,
    /// BuildGetDataCallList `0x0602` (proven empty GET).
    GetDataCallList,
    /// BuildSetupDataCall `0x0600` len 246 — only when APN resolved.
    SetupDataCall,
    /// Prove bearer: IPv4 on rmnet* and/or rx>0 && tx>0.
    VerifyRmnetBearer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostEdgePlan {
    /// Soft-lock still holds; START_NETWORK denied. Operator must reseat.
    SoftLockBlocked {
        app_state: u8,
        pin1: u8,
        reason: &'static str,
    },
    /// EDGE cleared (or already ready); run steps in order.
    Chase {
        app_state: u8,
        pin1: u8,
        steps: Vec<PostEdgeStep>,
        /// Non-secret APN hostname when SetupDataCall is in `steps`.
        apn: Option<String>,
        /// Why SetupDataCall was omitted (when apn is None).
        setup_data_call: SetupDataCallGate,
    },
}

/// Soft-lock gate for SetupDataCall (no invented APN / no send under PIN soft-lock).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupDataCallGate {
    /// Included in chase steps.
    Armed,
    /// Chase open but no usable APN config yet.
    DeferredNoApn,
    /// Soft-lock / START_NETWORK not open — never send.
    BlockedSoftLock,
}

/// CP START_NETWORK / camp gate: GET_APP must be DETECTED, PERSO, or READY.
pub fn start_network_allowed(app_state: u8) -> bool {
    matches!(
        app_state,
        APP_DETECTED | APP_SUBSCRIPTION_PERSO | APP_READY
    )
}

/// pin1 needs CardPower+VerifyPin before READY/camp path.
pub fn verify_pin_needed(pin1: u8) -> bool {
    matches!(pin1, 0 | 1)
}

/// Accept only safe APN host tokens (no secrets; no invent — caller supplies).
pub fn apn_is_usable(apn: &str) -> bool {
    let t = apn.trim();
    if t.is_empty() || t.len() > 100 {
        return false;
    }
    if t.eq_ignore_ascii_case("none") || t.eq_ignore_ascii_case("null") {
        return false;
    }
    // Hostname-ish: alnum, dot, hyphen, underscore. Reject spaces / URLs / creds.
    t.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && t.contains('.')
}

/// Parse APN from a one-line config file / `apn=…` blob (no secrets logged by caller).
pub fn resolve_apn_from_text(text: &str) -> Option<String> {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let value = if let Some(rest) = line.strip_prefix("apn=") {
            rest.trim()
        } else if let Some(rest) = line.strip_prefix("APN=") {
            rest.trim()
        } else {
            line
        };
        if apn_is_usable(value) {
            return Some(value.to_string());
        }
    }
    None
}

/// Full post-EDGE step list once gate opens.
pub fn chase_steps(include_verify_pin: bool, include_setup_data_call: bool) -> Vec<PostEdgeStep> {
    let mut steps = Vec::new();
    if include_verify_pin {
        steps.push(PostEdgeStep::VerifyPinIfNeeded);
    }
    steps.extend_from_slice(&[
        PostEdgeStep::GateStartNetwork,
        PostEdgeStep::RadioOn,
        PostEdgeStep::LteOnly,
        PostEdgeStep::NetworkSelectionAuto,
        PostEdgeStep::AllowData,
        PostEdgeStep::GetPsService,
        PostEdgeStep::PollDataRegistration,
        PostEdgeStep::GetDataCallList,
    ]);
    if include_setup_data_call {
        steps.push(PostEdgeStep::SetupDataCall);
    }
    steps.push(PostEdgeStep::VerifyRmnetBearer);
    steps
}

/// Plan from a soft-lock / SIM snapshot (host-safe). No APN → SetupDataCall deferred.
pub fn plan_from_snapshot(snap: SoftLockSnapshot) -> PostEdgePlan {
    plan_from_snapshot_with_apn(snap, None)
}

/// Plan with optional operator-supplied APN (already validated or raw).
pub fn plan_from_snapshot_with_apn(
    snap: SoftLockSnapshot,
    apn_raw: Option<&str>,
) -> PostEdgePlan {
    let apn = apn_raw.and_then(|s| {
        let t = s.trim();
        if apn_is_usable(t) {
            Some(t.to_string())
        } else {
            resolve_apn_from_text(s)
        }
    });

    if snap.is_modem06() {
        let reason = if snap.pin1 == PIN1_ENABLED_VERIFIED {
            "pin1_verified_chicken_egg: Pin1Verified OK but app still PIN; Present≠2"
        } else if snap.pin1 == PIN1_DISABLED {
            "pin1_disabled_soft_lock: no VerifyPin path; Present≠2"
        } else {
            "app_pin_blocks_start_network"
        };
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason,
        };
    }

    if snap.app_state == APP_STATE_PIN && !verify_pin_needed(snap.pin1) {
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason: "app_pin_blocks_start_network",
        };
    }

    let include_verify = verify_pin_needed(snap.pin1);
    let include_setup = apn.is_some();
    let gate = if include_setup {
        SetupDataCallGate::Armed
    } else {
        SetupDataCallGate::DeferredNoApn
    };

    // If still PIN but pin1∈{0,1}, EDGE VerifyPin window — include verify then gate.
    if snap.app_state == APP_STATE_PIN && include_verify {
        return PostEdgePlan::Chase {
            app_state: snap.app_state,
            pin1: snap.pin1,
            steps: chase_steps(true, include_setup),
            apn,
            setup_data_call: gate,
        };
    }

    if !start_network_allowed(snap.app_state) {
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason: "app_not_in_start_network_set_1_4_5",
        };
    }

    PostEdgePlan::Chase {
        app_state: snap.app_state,
        pin1: snap.pin1,
        steps: chase_steps(include_verify, include_setup),
        apn,
        setup_data_call: gate,
    }
}

/// Live bearer proof (IPv4 and/or bidirectional rmnet counters).
pub fn bearer_verified(ipv4: Option<&str>, rx: u64, tx: u64) -> bool {
    if let Some(a) = ipv4 {
        if !a.is_empty() && a != "none" {
            return true;
        }
    }
    rx > 0 && tx > 0
}

pub fn step_label(step: PostEdgeStep) -> &'static str {
    match step {
        PostEdgeStep::VerifyPinIfNeeded => "VerifyPin_A_AID_if_pin1_0_or_1",
        PostEdgeStep::GateStartNetwork => "gate_START_NETWORK_app_in_1_4_5",
        PostEdgeStep::RadioOn => "Radio_ON",
        PostEdgeStep::LteOnly => "LTE_ONLY_preferred_11",
        PostEdgeStep::NetworkSelectionAuto => "NetworkSelectionAuto_0x0704",
        PostEdgeStep::AllowData => "AllowData_0x0710",
        PostEdgeStep::GetPsService => "GetPsService_0x0711",
        PostEdgeStep::PollDataRegistration => "poll_data_reg_0x0701",
        PostEdgeStep::GetDataCallList => "GetDataCallList_0x0602",
        PostEdgeStep::SetupDataCall => "SetupDataCall_0x0600_len246_if_apn",
        PostEdgeStep::VerifyRmnetBearer => "verify_rmnet_ipv4_or_rxtx",
    }
}

pub fn advice_lines(plan: &PostEdgePlan) -> Vec<String> {
    match plan {
        PostEdgePlan::SoftLockBlocked {
            app_state,
            pin1,
            reason,
        } => vec![
            "post_edge=blocked".into(),
            format!("app_state={app_state}"),
            format!("pin1={pin1}"),
            format!("reason={reason}"),
            "start_network_allowed=no".into(),
            format!("cpif_caps_note={CPIF_CAPS_NOTE}"),
            "setup_data_call=blocked_soft_lock".into(),
            format!(
                "setup_data_call_gate={}",
                match SetupDataCallGate::BlockedSoftLock {
                    SetupDataCallGate::BlockedSoftLock => "blocked_soft_lock",
                    _ => unreachable!(),
                }
            ),
            "action=physical_tray_reseat_with_tray_bearer_chase".into(),
            "re_conclusion=READY_needs_Present_eq_2_only_FN_A_CDMA_EU_NoCDMA_RatMap".into(),
            "ratmap_lever=blocked_NV_TCS_only_preferred_cannot_add_CDMA".into(),
            "goal=incomplete_until_rmnet_ipv4".into(),
        ],
        PostEdgePlan::Chase {
            app_state,
            pin1,
            steps,
            apn,
            setup_data_call,
        } => {
            let mut lines = vec![
                "post_edge=chase".into(),
                format!("app_state={app_state}"),
                format!("pin1={pin1}"),
                format!(
                    "start_network_allowed={}",
                    if start_network_allowed(*app_state) {
                        "yes"
                    } else {
                        "after_VerifyPin_recheck"
                    }
                ),
                format!("cpif_caps_note={CPIF_CAPS_NOTE}"),
                format!("preferred_lte_only={PREFERRED_LTE_ONLY}"),
                format!("setup_data_call_id=0x{SETUP_DATA_CALL_ID:04x}"),
                format!("setup_data_call_len={SETUP_DATA_CALL_LEN_SIMPLE}"),
                format!("get_data_call_list_id=0x{GET_DATA_CALL_LIST_ID:04x}"),
                format!("setup_proto_default_ipv4v6={SETUP_PROTO_IPV4V6}"),
                format!("apn_config_path={APN_CONFIG_PATH}"),
            ];
            match setup_data_call {
                SetupDataCallGate::Armed => {
                    lines.push("setup_data_call=armed".into());
                    if let Some(a) = apn {
                        lines.push(format!("apn_len={}", a.len()));
                    }
                }
                SetupDataCallGate::DeferredNoApn => {
                    lines.push("setup_data_call=deferred_no_apn".into());
                    lines.push(format!(
                        "apn_hint=write_carrier_apn_to_{APN_CONFIG_PATH}_one_line"
                    ));
                }
                SetupDataCallGate::BlockedSoftLock => {
                    lines.push("setup_data_call=blocked_soft_lock".into());
                }
            }
            for (i, step) in steps.iter().enumerate() {
                lines.push(format!("step{}={}", i + 1, step_label(*step)));
            }
            lines.push("goal=incomplete_until_rmnet_ipv4".into());
            lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soft_lock::detect_from_status_text;

    #[test]
    fn start_network_set_is_1_4_5() {
        assert!(start_network_allowed(1));
        assert!(start_network_allowed(4));
        assert!(start_network_allowed(5));
        assert!(!start_network_allowed(2));
        assert!(!start_network_allowed(0));
        assert!(!start_network_allowed(6));
    }

    #[test]
    fn soft_lock_pin1_verified_blocks_chase() {
        let snap = SoftLockSnapshot {
            app_state: 2,
            pin1: PIN1_ENABLED_VERIFIED,
            present: None,
        };
        match plan_from_snapshot(snap) {
            PostEdgePlan::SoftLockBlocked { reason, .. } => {
                assert!(reason.contains("chicken_egg"));
            }
            other => panic!("expected blocked, got {other:?}"),
        }
        let advice = advice_lines(&plan_from_snapshot(snap));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=blocked_soft_lock")));
    }

    #[test]
    fn soft_lock_pin1_disabled_blocks_chase() {
        let snap = SoftLockSnapshot {
            app_state: 2,
            pin1: PIN1_DISABLED,
            present: None,
        };
        assert!(matches!(
            plan_from_snapshot(snap),
            PostEdgePlan::SoftLockBlocked { .. }
        ));
    }

    #[test]
    fn soft_lock_never_arms_setup_even_with_apn() {
        let snap = SoftLockSnapshot {
            app_state: 2,
            pin1: PIN1_ENABLED_VERIFIED,
            present: None,
        };
        match plan_from_snapshot_with_apn(snap, Some("internet.example.apn")) {
            PostEdgePlan::SoftLockBlocked { .. } => {}
            other => panic!("expected blocked, got {other:?}"),
        }
    }

    #[test]
    fn pin_not_verified_is_edge_chase_with_verify() {
        let snap = SoftLockSnapshot {
            app_state: 2,
            pin1: 1,
            present: None,
        };
        match plan_from_snapshot(snap) {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                ..
            } => {
                assert_eq!(steps[0], PostEdgeStep::VerifyPinIfNeeded);
                assert!(steps.contains(&PostEdgeStep::GateStartNetwork));
                assert!(steps.contains(&PostEdgeStep::LteOnly));
                assert!(steps.contains(&PostEdgeStep::AllowData));
                assert!(steps.contains(&PostEdgeStep::GetPsService));
                assert!(steps.contains(&PostEdgeStep::GetDataCallList));
                assert!(!steps.contains(&PostEdgeStep::SetupDataCall));
                assert_eq!(setup_data_call, SetupDataCallGate::DeferredNoApn);
                assert!(steps.contains(&PostEdgeStep::VerifyRmnetBearer));
            }
            other => panic!("expected chase, got {other:?}"),
        }
    }

    #[test]
    fn ready_without_apn_defers_setup() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: PIN1_ENABLED_VERIFIED,
            present: Some(2),
        };
        match plan_from_snapshot(snap) {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                apn,
                ..
            } => {
                assert!(!steps.contains(&PostEdgeStep::SetupDataCall));
                assert_eq!(setup_data_call, SetupDataCallGate::DeferredNoApn);
                assert!(apn.is_none());
                assert!(steps.contains(&PostEdgeStep::GetDataCallList));
            }
            other => panic!("expected chase, got {other:?}"),
        }
    }

    #[test]
    fn ready_with_apn_arms_setup_data_call() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: PIN1_ENABLED_VERIFIED,
            present: Some(2),
        };
        match plan_from_snapshot_with_apn(snap, Some("internet.example.apn")) {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                apn,
                ..
            } => {
                assert_eq!(setup_data_call, SetupDataCallGate::Armed);
                assert_eq!(apn.as_deref(), Some("internet.example.apn"));
                assert!(steps.contains(&PostEdgeStep::SetupDataCall));
                let setup_idx = steps
                    .iter()
                    .position(|s| *s == PostEdgeStep::SetupDataCall)
                    .unwrap();
                let list_idx = steps
                    .iter()
                    .position(|s| *s == PostEdgeStep::GetDataCallList)
                    .unwrap();
                assert!(list_idx < setup_idx);
                assert_eq!(*steps.last().unwrap(), PostEdgeStep::VerifyRmnetBearer);
            }
            other => panic!("expected chase, got {other:?}"),
        }
        let advice = advice_lines(&plan_from_snapshot_with_apn(
            SoftLockSnapshot {
                app_state: APP_READY,
                pin1: 2,
                present: Some(2),
            },
            Some("internet.example.apn"),
        ));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=armed")));
        assert!(advice.iter().any(|l| l.contains("SetupDataCall_0x0600")));
        assert!(advice.iter().any(|l| l.contains("apn_len=20")));
        // Never echo APN string into advice.
        assert!(!advice.iter().any(|l| l.contains("internet.example.apn")));
    }

    #[test]
    fn rejects_invented_or_unsafe_apn_tokens() {
        assert!(!apn_is_usable(""));
        assert!(!apn_is_usable("none"));
        assert!(!apn_is_usable("internet")); // no dot — too vague / not hostname
        assert!(!apn_is_usable("user:pass@host"));
        assert!(!apn_is_usable("http://evil"));
        assert!(apn_is_usable("internet.example.apn"));
        assert_eq!(
            resolve_apn_from_text("# c\napn=internet.example.apn\n"),
            Some("internet.example.apn".into())
        );
    }

    #[test]
    fn ready_app_plans_full_bearer_path() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: PIN1_ENABLED_VERIFIED,
            present: Some(2),
        };
        match plan_from_snapshot(snap) {
            PostEdgePlan::Chase { steps, .. } => {
                assert!(!steps.contains(&PostEdgeStep::VerifyPinIfNeeded));
                assert_eq!(steps[0], PostEdgeStep::GateStartNetwork);
                assert_eq!(*steps.last().unwrap(), PostEdgeStep::VerifyRmnetBearer);
            }
            other => panic!("expected chase, got {other:?}"),
        }
        let advice = advice_lines(&plan_from_snapshot(SoftLockSnapshot {
            app_state: APP_READY,
            pin1: 2,
            present: Some(2),
        }));
        assert!(advice.iter().any(|l| l.contains("start_network_allowed=yes")));
        assert!(advice.iter().any(|l| l.contains("GetPsService")));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=deferred_no_apn")));
    }

    #[test]
    fn detected_and_perso_also_start_network() {
        for app in [APP_DETECTED, APP_SUBSCRIPTION_PERSO] {
            let plan = plan_from_snapshot(SoftLockSnapshot {
                app_state: app,
                pin1: 2,
                present: Some(2),
            });
            assert!(matches!(plan, PostEdgePlan::Chase { .. }));
        }
    }

    #[test]
    fn bearer_needs_ipv4_or_bidirectional() {
        assert!(bearer_verified(Some("10.1.2.3/32"), 0, 0));
        assert!(bearer_verified(None, 100, 50));
        assert!(!bearer_verified(None, 0, 336)); // tx-only (live soft-lock shape)
        assert!(!bearer_verified(Some(""), 0, 0));
        assert!(!bearer_verified(Some("none"), 10, 0));
    }

    #[test]
    fn parses_live_soft_lock_log_into_blocked_plan() {
        let line = "SIM snapshot app=2 pin1=2 remain=3 card=1 present_infer=notin_1_2_3";
        let snap = detect_from_status_text(line).unwrap();
        let plan = plan_from_snapshot(snap);
        let advice = advice_lines(&plan);
        assert!(advice.iter().any(|l| l.contains("post_edge=blocked")));
        assert!(advice.iter().any(|l| l.contains("FN_A_CDMA")));
        assert!(advice.iter().any(|l| l.contains("ratmap_lever=blocked")));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=blocked_soft_lock")));
    }
}
