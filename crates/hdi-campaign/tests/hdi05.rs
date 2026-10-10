use hdi_campaign::{
    HardwareVerdict, IntentAuthor, IntentGate, ADAPT_HARDWARE, GRAPHICS_PRECONDITION,
    INVESTIGATE_HARDWARE, SHIPPED_PRECONDITIONS,
};
use intent_resolution::{
    AllowedContext, ContextSnapshot, IntentInput, IntentSource, ObjectSummary,
};
use saai_entity_store::ObjectRef;
use uuid::Uuid;

fn device() -> ObjectSummary {
    ObjectSummary {
        object: ObjectRef::entity(Uuid::from_u128(7)),
        entity_type: "saaios.device".into(),
        title: "Графика".into(),
        revision: 1,
    }
}

fn input(text: &str, explicit: Option<&str>) -> (IntentInput, AllowedContext) {
    let summary = device();
    let allowed =
        AllowedContext::default().with_object(&summary, vec![INVESTIGATE_HARDWARE.to_string()]);
    let intent = IntentInput {
        text: text.into(),
        source: IntentSource::Orb,
        explicit_action_id: explicit.map(str::to_string),
        primary_object: Some(summary.object.clone()),
        context: ContextSnapshot {
            primary_space_id: Some("system".into()),
            active_space_ids: vec!["system".into()],
            focused_object: Some(summary),
            selected_objects: Vec::new(),
        },
        plan: None,
    };
    (intent, allowed)
}

#[test]
fn explicit_intent_starts_one_campaign() {
    let mut gate = IntentGate::new();
    let (intent, allowed) = input("исследуй графику", Some(INVESTIGATE_HARDWARE));
    let decision = gate.submit(&intent, &allowed);
    assert!(decision.started_campaign);
    assert_eq!(
        decision.resolved_action.as_deref(),
        Some(INVESTIGATE_HARDWARE)
    );
    assert_eq!(gate.intents().len(), 1);
    assert_eq!(gate.campaigns().len(), 1);
    assert_eq!(gate.campaigns()[0].intent_index, 0);
    assert_eq!(gate.intents()[0].author, IntentAuthor::Orb);
}

#[test]
fn free_text_does_not_start_hdi() {
    let mut gate = IntentGate::new();
    let (intent, allowed) = input("разберись с графикой ноутбука", None);
    let decision = gate.submit(&intent, &allowed);
    assert!(!decision.started_campaign);
    assert_ne!(
        decision.resolved_action.as_deref(),
        Some(INVESTIGATE_HARDWARE)
    );
    assert!(gate.campaigns().is_empty());
}

#[test]
fn boot_fixture_creates_no_campaign() {
    let mut gate = IntentGate::new();
    assert_eq!(gate.observe("boot", "cold-start"), 0);
    assert!(gate.intents().is_empty());
    assert!(gate.campaigns().is_empty());
}

#[test]
fn disabled_precondition_creates_no_intent() {
    assert!(!SHIPPED_PRECONDITIONS[0].enabled);
    assert_eq!(SHIPPED_PRECONDITIONS[0].id, GRAPHICS_PRECONDITION);
    let mut gate = IntentGate::new();
    assert_eq!(gate.observe(GRAPHICS_PRECONDITION, "no-igd"), 0);
    assert!(gate.intents().is_empty());
}

#[test]
fn enabled_precondition_matches_once() {
    let mut gate = IntentGate::new();
    gate.enable_for_test(GRAPHICS_PRECONDITION);
    assert_eq!(gate.observe(GRAPHICS_PRECONDITION, "no-igd"), 1);
    assert_eq!(gate.observe(GRAPHICS_PRECONDITION, "no-igd"), 0);
    assert_eq!(gate.intents().len(), 1);
    assert_eq!(gate.campaigns().len(), 1);
    assert!(!SHIPPED_PRECONDITIONS[0].enabled);
}

#[test]
fn worker_intent_names_precondition() {
    let mut gate = IntentGate::new();
    gate.enable_for_test(GRAPHICS_PRECONDITION);
    gate.observe(GRAPHICS_PRECONDITION, "no-igd");
    let intent = &gate.intents()[0];
    assert_eq!(intent.author, IntentAuthor::Worker);
    assert_eq!(
        intent.precondition_id.as_deref(),
        Some(GRAPHICS_PRECONDITION)
    );
    assert_eq!(intent.semantic_action_id, INVESTIGATE_HARDWARE);
}

#[test]
fn adapt_hardware_asks_user() {
    let gate = IntentGate::new();
    let (verdict, adaptation) = gate.request_adaptation(ADAPT_HARDWARE);
    assert_eq!(verdict, HardwareVerdict::AskUser);
    assert!(!adaptation.loaded_module);
    let (again, adaptation) = gate.request_adaptation(ADAPT_HARDWARE);
    assert_eq!(again, HardwareVerdict::AskUser);
    assert!(!adaptation.loaded_module);
    assert!(gate.campaigns().is_empty());
}

#[test]
fn unknown_action_still_unsupported() {
    let mut gate = IntentGate::new();
    let (intent, allowed) = input("форматируй", Some("format_disk"));
    let decision = gate.submit(&intent, &allowed);
    assert!(!decision.started_campaign);
    assert!(decision.resolved_action.is_none());
    assert!(gate.campaigns().is_empty());
}

#[test]
fn native_init_does_not_start_hdi() {
    let init = include_str!("../../../os/targets/panther/src/native-init.c");
    assert!(!init.contains("hdi-campaign"));
    assert!(!init.contains("investigate_hardware"));
    assert!(!init.contains("graphics_class_incomplete"));
}
