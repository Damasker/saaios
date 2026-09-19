//! ADR-180: names `.sui` v2 may eventually compile.
//!
//! This is not a parser. `compile()` still accepts only `sui 1`.
//! Space detail, Memory review, chat, and widgets are omitted.

/// Primitive contracts from `saai-ui-core`. PascalCase matches the
/// Rust type. Not a markup keyword table.
pub fn sui_v2_primitives() -> &'static [&'static str] {
    &[
        "SemanticText",
        "Icon",
        "Divider",
        "StatusIndicator",
        "Progress",
        "Button",
        "Field",
        "DataRow",
        "Metric",
        "Disclosure",
        "SurfacePattern",
    ]
}

/// Composite contracts from `saai-ui-core`. First real consumers are
/// on panther except gallery fixtures that stay host-labelled.
pub fn sui_v2_composites() -> &'static [&'static str] {
    &[
        "ContextHeader",
        "SystemSection",
        "ObjectSummary",
        "BottomNavigation",
        "OrbHost",
        "SystemStatus",
        "IntentSummary",
        "TaskSummary",
        "DecisionOverlay",
        "AgentSummary",
        "SettingRow",
        "CapabilityRow",
        "EventRow",
        "SpaceRow",
        "WifiRow",
        "BluetoothRow",
        "TrustedClientRow",
    ]
}

/// Live shell surfaces. Ids, not Rust types. Space detail is omitted.
pub fn sui_v2_surfaces() -> &'static [&'static str] {
    &[
        "now",
        "inbox",
        "spaces",
        "me",
        "object",
        "intent",
        "apps",
        "wifi",
        "bluetooth",
        "trusted",
        "wifi-password",
        "pin-setup",
        "lock",
        "consent",
        "remote-pair",
        "diagnostic",
        "gallery",
    ]
}

/// Named so a later grammar cannot smuggle them in as "missing v2".
pub fn sui_v2_deferred() -> &'static [&'static str] {
    &["SpaceDetail", "MemoryReview", "ChatThread", "Widget"]
}

/// May appear in shell `.sui` later; not a third-party subset.
pub fn sui_v2_privileged() -> &'static [&'static str] {
    &[
        "OrbHost",
        "SystemStatus",
        "DecisionOverlay",
        "CapabilityRow",
        "TrustedClientRow",
        "lock",
        "diagnostic",
        "gallery",
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        sui_v2_composites, sui_v2_deferred, sui_v2_primitives, sui_v2_privileged, sui_v2_surfaces,
    };
    use std::collections::HashSet;

    #[test]
    fn vocabulary_is_the_proven_core_types_and_live_surfaces() {
        assert_eq!(sui_v2_primitives().len(), 11);
        assert_eq!(sui_v2_composites().len(), 17);
        assert_eq!(sui_v2_surfaces().len(), 17);
        assert!(sui_v2_primitives().contains(&"Field"));
        assert!(sui_v2_primitives().contains(&"SurfacePattern"));
        assert!(sui_v2_composites().contains(&"EventRow"));
        assert!(sui_v2_composites().contains(&"TrustedClientRow"));
        assert!(sui_v2_surfaces().contains(&"now"));
        assert!(sui_v2_surfaces().contains(&"me"));
        assert!(sui_v2_surfaces().contains(&"lock"));
    }

    #[test]
    fn deferred_names_are_outside_the_vocabulary() {
        let named: HashSet<&str> = sui_v2_primitives()
            .iter()
            .chain(sui_v2_composites())
            .chain(sui_v2_surfaces())
            .copied()
            .collect();
        for deferred in sui_v2_deferred() {
            assert!(!named.contains(deferred), "{deferred} leaked into v2");
        }
        assert!(!named.contains(&"SpaceDetail"));
        assert!(!named.contains(&"MemoryReview"));
    }

    #[test]
    fn privileged_names_are_a_subset_of_the_vocabulary() {
        let named: HashSet<&str> = sui_v2_composites()
            .iter()
            .chain(sui_v2_surfaces())
            .copied()
            .collect();
        for name in sui_v2_privileged() {
            assert!(named.contains(name), "{name} is privileged but unnamed");
        }
    }
}
