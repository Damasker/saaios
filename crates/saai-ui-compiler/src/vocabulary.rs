//! ADR-180/181: names `.sui` v2 may compile.
//!
//! `compile()` still accepts only `sui 1`. `compile_v2()` uses these
//! lists. Space detail, Memory review, chat, and widgets are omitted.

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

pub fn sui_v2_is_component(name: &str) -> bool {
    sui_v2_primitives().contains(&name) || sui_v2_composites().contains(&name)
}

pub fn sui_v2_is_surface(id: &str) -> bool {
    sui_v2_surfaces().contains(&id)
}

pub fn sui_v2_is_deferred(name: &str) -> bool {
    sui_v2_deferred().contains(&name)
}

pub fn sui_v2_is_privileged(name: &str) -> bool {
    sui_v2_privileged().contains(&name)
}

/// ADR-185: third-party names. Privileged shell chrome is excluded.
pub fn sui_v2_is_public(name: &str) -> bool {
    (sui_v2_is_component(name) || sui_v2_is_surface(name)) && !sui_v2_is_privileged(name)
}

/// ADR-186: published stability labels. Nothing is Stable yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuiV2Stability {
    Experimental,
    Privileged,
    Deferred,
}

pub fn sui_v2_stability(name: &str) -> Option<SuiV2Stability> {
    if sui_v2_is_deferred(name) {
        Some(SuiV2Stability::Deferred)
    } else if sui_v2_is_privileged(name) {
        Some(SuiV2Stability::Privileged)
    } else if sui_v2_is_public(name) {
        Some(SuiV2Stability::Experimental)
    } else {
        None
    }
}

pub fn sui_v2_property_keys() -> &'static [&'static str] {
    &[
        "text", "color", "spacing", "inset", "scroll", "loc", "focus", "a11y",
    ]
}

pub fn sui_v2_text_roles() -> &'static [&'static str] {
    &[
        "Display", "Title", "Section", "Body", "Label", "Caption", "MonoBody",
    ]
}

pub fn sui_v2_color_roles() -> &'static [&'static str] {
    &[
        "Canvas",
        "Surface",
        "Elevated",
        "Accent",
        "AccentHighlight",
        "TextPrimary",
        "TextSecondary",
        "Success",
        "Attention",
        "Critical",
        "Border",
        "Grid",
        "Pressed",
        "Focus",
        "DisabledSurface",
        "DisabledText",
        "HighContrastText",
    ]
}

pub fn sui_v2_spacing_tokens() -> &'static [&'static str] {
    &[
        "None", "XSmall", "Small", "Medium", "Large", "XLarge", "XXLarge",
    ]
}

pub fn sui_v2_inset_values() -> &'static [&'static str] {
    &["none", "safe"]
}

pub fn sui_v2_scroll_values() -> &'static [&'static str] {
    &["none", "region"]
}

pub fn sui_v2_a11y_roles() -> &'static [&'static str] {
    &[
        "Text",
        "Heading",
        "Image",
        "Button",
        "TextField",
        "ListItem",
        "Disclosure",
        "ProgressIndicator",
        "Status",
        "Dialog",
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        sui_v2_a11y_roles, sui_v2_color_roles, sui_v2_composites, sui_v2_deferred,
        sui_v2_inset_values, sui_v2_is_component, sui_v2_is_deferred, sui_v2_is_privileged,
        sui_v2_is_public, sui_v2_is_surface, sui_v2_primitives, sui_v2_privileged,
        sui_v2_property_keys, sui_v2_scroll_values, sui_v2_spacing_tokens, sui_v2_stability,
        sui_v2_surfaces, sui_v2_text_roles, SuiV2Stability,
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
            assert!(
                !sui_v2_is_public(name),
                "{name} leaked into the public subset"
            );
        }
        for primitive in sui_v2_primitives() {
            assert!(sui_v2_is_public(primitive), "{primitive} must be public");
        }
    }

    #[test]
    fn lookup_helpers_follow_the_lists() {
        assert!(sui_v2_is_component("ContextHeader"));
        assert!(sui_v2_is_component("Field"));
        assert!(sui_v2_is_surface("now"));
        assert!(sui_v2_is_surface("wifi-password"));
        assert!(sui_v2_is_privileged("OrbHost"));
        assert!(sui_v2_is_deferred("SpaceDetail"));
        assert!(sui_v2_is_public("ContextHeader"));
        assert!(sui_v2_is_public("now"));
        assert!(!sui_v2_is_public("OrbHost"));
        assert!(!sui_v2_is_public("lock"));
        assert!(!sui_v2_is_public("SpaceDetail"));
        assert_eq!(
            sui_v2_stability("ContextHeader"),
            Some(SuiV2Stability::Experimental)
        );
        assert_eq!(
            sui_v2_stability("OrbHost"),
            Some(SuiV2Stability::Privileged)
        );
        assert_eq!(
            sui_v2_stability("SpaceDetail"),
            Some(SuiV2Stability::Deferred)
        );
        assert_eq!(sui_v2_stability("WidgetCard"), None);
        assert!(!sui_v2_is_component("SpaceDetail"));
        assert!(!sui_v2_is_surface("root"));
        assert_eq!(sui_v2_property_keys().len(), 8);
        assert!(sui_v2_text_roles().contains(&"Title"));
        assert!(sui_v2_color_roles().contains(&"TextPrimary"));
        assert!(sui_v2_spacing_tokens().contains(&"Medium"));
        assert_eq!(sui_v2_inset_values(), &["none", "safe"]);
        assert_eq!(sui_v2_scroll_values(), &["none", "region"]);
        assert!(sui_v2_a11y_roles().contains(&"Heading"));
        assert!(!sui_v2_text_roles().contains(&"Headline"));
    }
}
