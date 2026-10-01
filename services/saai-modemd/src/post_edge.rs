//! Post-EDGE bearer pipeline (pure; no device I/O).
//!
//! The published SIM application state gates START_NETWORK. On this Panther,
//! tested tray reseat and a bounded passive wait left that state at PIN in
//! their test windows. A concurrent diagnostic twice reached READY from
//! PIN/pin1=1 using VerifyPin A+AID without CardPower. It held the SIT channel
//! during an RFS test, so that test cannot establish self-init or RFS
//! causality. The current CP Present byte is not exposed by SIT status;
//! catalog id `0x2f50` is internal to the CP.
//!
//! SetupDataCall (`0x0600`): sit-stream simple builder length **246**
//! (`0xf6`); extended TD overload length **983** (`0x3d7`). Gated by
//! `isValidPdpApn` (non-NULL PdpContext + field). APN copied to packet
//! offset 16 (max 100). Send only after registration 1/5. Never invent an APN string — resolve from
//! operator config (`/data/saaios/etc/apn`) or explicit CLI.
//!
//! The guarded runtime runner requires a fresh READY(5) and radio ON(10).
//! CP START_NETWORK also accepts app 1/4, but this runner does not arm on them.

use crate::soft_lock::{
    SoftLockSnapshot, APP_STATE_PIN, BLOCKER_APP_PIN, CPIF_CAPS_NOTE, PIN1_DISABLED,
    PIN1_ENABLED_VERIFIED,
};

/// DETECTED — START_NETWORK accepts.
pub const APP_DETECTED: u8 = 1;
/// SUBSCRIPTION_PERSO — START_NETWORK accepts.
pub const APP_SUBSCRIPTION_PERSO: u8 = 4;
/// READY — START_NETWORK accepts.
pub const APP_READY: u8 = 5;

/// Evidenced simple `BuildSetupDataCall` frame length (sit-stream `0x78c80`).
pub const SETUP_DATA_CALL_LEN_SIMPLE: u16 = 246;
/// Evidenced SIT id for SetupDataCall.
pub const SETUP_DATA_CALL_ID: u16 = 0x0600;
/// Factory RO default when DB has no protocol: IPV4V6.
pub const SETUP_PROTO_IPV4V6: u8 = 3;
/// On-device APN path (operator-supplied; never invent carrier string).
pub const APN_CONFIG_PATH: &str = "/data/saaios/etc/apn";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostEdgeStep {
    /// Explicitly opted-in VerifyPin A+AID when app=PIN, pin1∈{0,1}, remain>1.
    VerifyPinIfNeeded,
    /// The guarded runner requires READY(5) after any PIN verification.
    GateStartNetwork,
    /// Guarded runner requires radio ON; it does not power-cycle the radio.
    RadioOn,
    /// Guarded runner checks mode and sends auto `0x0704` only if manual.
    NetworkSelectionAuto,
    /// Guarded runner sends AllowData `0x0710` allow=1 once.
    AllowData,
    /// Poll data-registration `0x0701` until home(1) or roaming(5).
    PollDataRegistration,
    /// BuildSetupDataCall `0x0600` len 246 — only when APN resolved and reg=1/5.
    SetupDataCall,
    /// Prove bearer: IPv4 on rmnet* and/or rx>0 && tx>0.
    VerifyRmnetBearer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostEdgePlan {
    /// The guarded network runner cannot start from this app state.
    SoftLockBlocked {
        app_state: u8,
        pin1: u8,
        reason: &'static str,
    },
    /// READY, or an explicitly opted-in conditional PIN→READY plan.
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
    /// Included in chase steps; runtime still requires registration 1/5.
    Armed,
    /// Chase open but no usable APN config yet.
    DeferredNoApn,
    /// APN is available, but the separate data-call opt-in was not given.
    DeferredOptIn,
    /// Soft-lock / START_NETWORK not open — never send.
    BlockedSoftLock,
}

/// Host-side planning permissions only. These never perform device I/O.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostEdgeOptIns {
    pub allow_pin_verify: bool,
    pub allow_setup_data_call: bool,
}

/// CP START_NETWORK / camp gate: GET_APP must be DETECTED, PERSO, or READY.
pub fn start_network_allowed(app_state: u8) -> bool {
    matches!(
        app_state,
        APP_DETECTED | APP_SUBSCRIPTION_PERSO | APP_READY
    )
}

/// PIN verification could be considered only with a separate explicit opt-in.
pub fn verify_pin_needed(pin1: u8) -> bool {
    matches!(pin1, 0 | 1)
}

/// Accept only safe APN host tokens (no secrets; no invent — caller supplies).
pub fn apn_is_usable(apn: &str) -> bool {
    let t = apn.trim();
    if t.is_empty() || t.len() > 100 {
        return false;
    }
    if ["none", "null", "unknown", "unset"]
        .iter()
        .any(|placeholder| t.eq_ignore_ascii_case(placeholder))
    {
        return false;
    }
    // Carrier APNs can be one label (e.g. "internet") or dotted labels.
    // Accept only ASCII token characters; never accept a URL or credentials.
    t.split('.').all(|label| {
        !label.is_empty()
            && label.chars().any(|c| c.is_ascii_alphanumeric())
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    })
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

/// Conditional post-EDGE step list; the runtime rechecks READY and reg=1/5.
pub fn chase_steps(include_verify_pin: bool, include_setup_data_call: bool) -> Vec<PostEdgeStep> {
    let mut steps = Vec::new();
    if include_verify_pin {
        steps.push(PostEdgeStep::VerifyPinIfNeeded);
    }
    steps.extend_from_slice(&[
        PostEdgeStep::GateStartNetwork,
        PostEdgeStep::RadioOn,
        PostEdgeStep::NetworkSelectionAuto,
        PostEdgeStep::AllowData,
        PostEdgeStep::PollDataRegistration,
    ]);
    if include_setup_data_call {
        steps.push(PostEdgeStep::SetupDataCall);
    }
    steps.push(PostEdgeStep::VerifyRmnetBearer);
    steps
}

/// Conservative host-safe plan: PIN verification and data call are disarmed.
pub fn plan_from_snapshot(snap: SoftLockSnapshot) -> PostEdgePlan {
    plan_from_snapshot_with_apn(snap, None)
}

/// Conservative host-safe plan with APN, but no permission to use it yet.
pub fn plan_from_snapshot_with_apn(
    snap: SoftLockSnapshot,
    apn_raw: Option<&str>,
) -> PostEdgePlan {
    plan_from_snapshot_with_opt_ins(snap, apn_raw, PostEdgeOptIns::default())
}

/// Plan explicitly permitted actions. On-device tools still enforce fresh
/// status, remaining PIN attempts, and data registration independently.
pub fn plan_from_snapshot_with_opt_ins(
    snap: SoftLockSnapshot,
    apn_raw: Option<&str>,
    opt_ins: PostEdgeOptIns,
) -> PostEdgePlan {
    let apn = apn_raw.and_then(|s| {
        let t = s.trim();
        if apn_is_usable(t) {
            Some(t.to_string())
        } else {
            resolve_apn_from_text(s)
        }
    });

    let include_verify = snap.app_state == APP_STATE_PIN
        && verify_pin_needed(snap.pin1)
        && opt_ins.allow_pin_verify;
    let include_setup = apn.is_some() && opt_ins.allow_setup_data_call;
    let gate = if include_setup {
        SetupDataCallGate::Armed
    } else if apn.is_some() {
        SetupDataCallGate::DeferredOptIn
    } else {
        SetupDataCallGate::DeferredNoApn
    };

    // Even with host-side opt-in, the on-device watcher makes a fresh check
    // and permits no more than one PIN attempt per process.
    if include_verify {
        return PostEdgePlan::Chase {
            app_state: snap.app_state,
            pin1: snap.pin1,
            steps: chase_steps(true, include_setup),
            apn,
            setup_data_call: gate,
        };
    }

    if snap.is_modem06() {
        let reason = if verify_pin_needed(snap.pin1) {
            "verify_pin_explicit_opt_in_required"
        } else if snap.pin1 == PIN1_ENABLED_VERIFIED {
            "pin1_verified_chicken_egg: Pin1Verified OK but app still PIN; Present unknown"
        } else if snap.pin1 == PIN1_DISABLED {
            "pin1_disabled_soft_lock: no VerifyPin path; Present unknown"
        } else {
            "app_pin_blocks_start_network"
        };
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason,
        };
    }

    if !start_network_allowed(snap.app_state) {
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason: "app_not_in_start_network_set_1_4_5",
        };
    }

    if snap.app_state != APP_READY {
        return PostEdgePlan::SoftLockBlocked {
            app_state: snap.app_state,
            pin1: snap.pin1,
            reason: "guarded_network_runner_requires_ready_5",
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
        PostEdgeStep::VerifyPinIfNeeded => "VerifyPin_A_AID_if_pin1_0_or_1_explicit_opt_in",
        PostEdgeStep::GateStartNetwork => "gate_guarded_runner_READY_5",
        PostEdgeStep::RadioOn => "guarded_runner_requires_radio_ON_10",
        PostEdgeStep::NetworkSelectionAuto => "guarded_runner_auto_if_manual_0x0704",
        PostEdgeStep::AllowData => "guarded_runner_allowdata_once_0x0710",
        PostEdgeStep::PollDataRegistration => "poll_data_reg_0x0701",
        PostEdgeStep::SetupDataCall => "SetupDataCall_0x0600_len246_if_apn_and_reg_1_or_5",
        PostEdgeStep::VerifyRmnetBearer => "verify_rmnet_ipv4_or_rxtx",
    }
}

pub fn advice_lines(plan: &PostEdgePlan) -> Vec<String> {
    match plan {
        PostEdgePlan::SoftLockBlocked {
            app_state,
            pin1,
            reason,
        } => {
            let is_pin = *app_state == APP_STATE_PIN;
            let gate = if is_pin { "blocked_soft_lock" } else { "blocked_no_ready" };
            let blocker = if is_pin {
                BLOCKER_APP_PIN
            } else {
                "guarded_network_runner_requires_ready_5"
            };
            let pin_verify = if is_pin && verify_pin_needed(*pin1) {
                "deferred_explicit_opt_in_required"
            } else {
                "not_applicable"
            };
            vec![
                "post_edge=blocked".into(),
                format!("app_state={app_state}"),
                format!("pin1={pin1}"),
                format!("reason={reason}"),
                format!("start_network_allowed={}", if start_network_allowed(*app_state) { "yes" } else { "no" }),
                "guarded_runner_allowed=no".into(),
                "cpif_caps_exercised=yes".into(),
                format!("cpif_caps_note={CPIF_CAPS_NOTE}"),
                format!("blocker={blocker}"),
                "present_status=unknown_not_on_sit_wire".into(),
                "panther_observed=verify_pin_a_aid_without_cardpower_ready_twice".into(),
                "rfs_causality=unproven_concurrent_verify_pin_sit_owner".into(),
                format!("pin_verify={pin_verify}"),
                format!("setup_data_call={gate}"),
                format!("setup_data_call_gate={gate}"),
                "action=repeat_fresh_sim_status_then_chase_if_ready".into(),
                "goal=incomplete_until_rmnet_ipv4".into(),
            ]
        }
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
                "preferred_policy=read_without_forcing_lte_only".into(),
                "runtime_runner=/data/saaios/bin/ready-network-once".into(),
                "runtime_runner_arg=run".into(),
                format!("setup_data_call_id=0x{SETUP_DATA_CALL_ID:04x}"),
                format!("setup_data_call_len={SETUP_DATA_CALL_LEN_SIMPLE}"),
                "setup_data_call_runtime_gate=registration_1_or_5".into(),
                format!("setup_proto_default_ipv4v6={SETUP_PROTO_IPV4V6}"),
                format!("apn_config_path={APN_CONFIG_PATH}"),
            ];
            lines.push(format!(
                "pin_verify={}",
                if steps.contains(&PostEdgeStep::VerifyPinIfNeeded) {
                    "opted_in_conditional"
                } else {
                    "not_needed"
                }
            ));
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
                SetupDataCallGate::DeferredOptIn => {
                    lines.push("setup_data_call=deferred_explicit_opt_in_required".into());
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
        assert!(advice.iter().any(|l| l.contains("cpif_caps_exercised=yes")));
        assert!(advice.iter().any(|l| l.contains(BLOCKER_APP_PIN)));
        assert!(advice.iter().any(|l| l.contains("present_status=unknown")));
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
    fn pin_not_verified_requires_explicit_opt_in() {
        let snap = SoftLockSnapshot {
            app_state: 2,
            pin1: 1,
            present: None,
        };
        assert!(matches!(
            plan_from_snapshot(snap),
            PostEdgePlan::SoftLockBlocked {
                reason: "verify_pin_explicit_opt_in_required",
                ..
            }
        ));
        let advice = advice_lines(&plan_from_snapshot(snap));
        assert!(advice
            .iter()
            .any(|line| line == "pin_verify=deferred_explicit_opt_in_required"));
        match plan_from_snapshot_with_opt_ins(
            snap,
            None,
            PostEdgeOptIns {
                allow_pin_verify: true,
                allow_setup_data_call: false,
            },
        ) {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                ..
            } => {
                assert_eq!(steps[0], PostEdgeStep::VerifyPinIfNeeded);
                assert!(steps.contains(&PostEdgeStep::GateStartNetwork));
                assert!(steps.contains(&PostEdgeStep::NetworkSelectionAuto));
                assert!(steps.contains(&PostEdgeStep::AllowData));
                assert!(steps.contains(&PostEdgeStep::PollDataRegistration));
                assert!(!steps.contains(&PostEdgeStep::SetupDataCall));
                assert_eq!(setup_data_call, SetupDataCallGate::DeferredNoApn);
                assert!(steps.contains(&PostEdgeStep::VerifyRmnetBearer));
            }
            other => panic!("expected chase, got {other:?}"),
        }
    }

    #[test]
    fn pin_verify_opt_in_does_not_also_arm_data_call() {
        let snap = SoftLockSnapshot {
            app_state: APP_STATE_PIN,
            pin1: 1,
            present: None,
        };
        let plan = plan_from_snapshot_with_opt_ins(
            snap,
            Some("internet"),
            PostEdgeOptIns {
                allow_pin_verify: true,
                allow_setup_data_call: false,
            },
        );
        let PostEdgePlan::Chase { steps, setup_data_call, .. } = plan else {
            panic!("explicit PIN plan should remain conditional");
        };
        assert!(steps.contains(&PostEdgeStep::VerifyPinIfNeeded));
        assert!(!steps.contains(&PostEdgeStep::SetupDataCall));
        assert_eq!(setup_data_call, SetupDataCallGate::DeferredOptIn);
    }

    #[test]
    fn ready_app_never_repeats_pin_verification() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: 1,
            present: None,
        };
        let plan = plan_from_snapshot_with_opt_ins(
            snap,
            None,
            PostEdgeOptIns {
                allow_pin_verify: true,
                allow_setup_data_call: false,
            },
        );
        let PostEdgePlan::Chase { steps, .. } = plan else {
            panic!("READY should plan a network diagnostic");
        };
        assert!(!steps.contains(&PostEdgeStep::VerifyPinIfNeeded));
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
                assert!(steps.contains(&PostEdgeStep::PollDataRegistration));
            }
            other => panic!("expected chase, got {other:?}"),
        }
    }

    #[test]
    fn ready_with_apn_requires_separate_data_call_opt_in() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: PIN1_ENABLED_VERIFIED,
            present: Some(2),
        };
        match plan_from_snapshot_with_apn(snap, Some("internet.example.apn")) {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                ..
            } => {
                assert_eq!(setup_data_call, SetupDataCallGate::DeferredOptIn);
                assert!(!steps.contains(&PostEdgeStep::SetupDataCall));
            }
            other => panic!("expected deferred chase, got {other:?}"),
        }
        let opted_in_plan = plan_from_snapshot_with_opt_ins(
            snap,
            Some("internet.example.apn"),
            PostEdgeOptIns {
                allow_pin_verify: false,
                allow_setup_data_call: true,
            },
        );
        match &opted_in_plan {
            PostEdgePlan::Chase {
                steps,
                setup_data_call,
                apn,
                ..
            } => {
                assert_eq!(*setup_data_call, SetupDataCallGate::Armed);
                assert_eq!(apn.as_deref(), Some("internet.example.apn"));
                assert!(steps.contains(&PostEdgeStep::SetupDataCall));
                let setup_idx = steps
                    .iter()
                    .position(|s| *s == PostEdgeStep::SetupDataCall)
                    .unwrap();
                let registration_idx = steps
                    .iter()
                    .position(|s| *s == PostEdgeStep::PollDataRegistration)
                    .unwrap();
                assert!(registration_idx < setup_idx);
                assert_eq!(*steps.last().unwrap(), PostEdgeStep::VerifyRmnetBearer);
            }
            other => panic!("expected chase, got {other:?}"),
        }
        let advice = advice_lines(&opted_in_plan);
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
        assert!(apn_is_usable("internet"));
        assert!(!apn_is_usable("unknown"));
        assert!(!apn_is_usable("unset"));
        assert!(!apn_is_usable("in ternet"));
        assert!(!apn_is_usable("foo..bar"));
        assert!(!apn_is_usable(".internet"));
        assert!(!apn_is_usable("user:pass@host"));
        assert!(!apn_is_usable("http://evil"));
        assert!(apn_is_usable("internet.example.apn"));
        assert_eq!(
            resolve_apn_from_text("# c\napn=internet.example.apn\n"),
            Some("internet.example.apn".into())
        );
    }
    #[test]
    fn single_label_apn_is_planned_after_registration_poll() {
        let snap = SoftLockSnapshot {
            app_state: APP_READY,
            pin1: PIN1_ENABLED_VERIFIED,
            present: None,
        };
        let plan = plan_from_snapshot_with_opt_ins(
            snap,
            Some("internet"),
            PostEdgeOptIns {
                allow_pin_verify: false,
                allow_setup_data_call: true,
            },
        );
        let PostEdgePlan::Chase { steps, apn, .. } = &plan else {
            panic!("READY with operator APN should plan a chase");
        };
        assert_eq!(apn.as_deref(), Some("internet"));
        let reg = steps.iter().position(|s| *s == PostEdgeStep::PollDataRegistration).unwrap();
        let setup = steps.iter().position(|s| *s == PostEdgeStep::SetupDataCall).unwrap();
        assert!(reg < setup);
        assert!(step_label(steps[setup]).contains("reg_1_or_5"));
        let advice = advice_lines(&plan);
        assert!(advice.iter().any(|line| line == "setup_data_call_runtime_gate=registration_1_or_5"));
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
        assert!(advice.iter().any(|l| l.contains("poll_data_reg_0x0701")));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=deferred_no_apn")));
    }

    #[test]
    fn detected_and_perso_are_cp_allowed_but_not_armed_by_guarded_runner() {
        for app in [APP_DETECTED, APP_SUBSCRIPTION_PERSO] {
            assert!(start_network_allowed(app));
            let plan = plan_from_snapshot(SoftLockSnapshot {
                app_state: app,
                pin1: 2,
                present: Some(2),
            });
            assert!(matches!(plan, PostEdgePlan::SoftLockBlocked { reason: "guarded_network_runner_requires_ready_5", .. }));
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
        assert!(advice.iter().any(|l| l.contains("verify_pin_a_aid_without_cardpower_ready_twice")));
        assert!(advice.iter().any(|l| l.contains("rfs_causality=unproven")));
        assert!(advice.iter().any(|l| l.contains("chase_if_ready")));
        assert!(advice.iter().any(|l| l.contains("setup_data_call=blocked_soft_lock")));
    }
}
