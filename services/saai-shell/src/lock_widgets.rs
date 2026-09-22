//! ADR-425: trusted system disclosure, never an application authorization grant.
//! Inputs deliberately cannot carry user text, object identifiers or actions.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockWidgetPolicy {
    Hidden,
    Device,
    #[default]
    Summary,
}

impl LockWidgetPolicy {
    pub fn from_setting(value: Option<&serde_json::Value>) -> Self {
        match value {
            None => Self::default(),
            Some(serde_json::Value::String(value)) if value == "summary" => Self::Summary,
            Some(serde_json::Value::String(value)) if value == "device" => Self::Device,
            _ => Self::Hidden,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Device => "device",
            Self::Summary => "summary",
        }
    }

    pub fn project(
        self,
        sleeping: bool,
        has_pin: bool,
        battery: Option<(u8, bool)>,
        store_connected: bool,
        has_attention: bool,
    ) -> LockWidgets {
        if sleeping || has_pin || self == Self::Hidden {
            return LockWidgets::default();
        }
        LockWidgets {
            battery: battery.filter(|(percent, _)| *percent <= 100),
            attention: (self == Self::Summary).then_some((store_connected, has_attention)),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct LockWidgets {
    pub battery: Option<(u8, bool)>,
    /// Connectivity and existence only, never entity content.
    pub attention: Option<(bool, bool)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_setting_preserves_existing_behavior_but_invalid_hides() {
        assert_eq!(
            LockWidgetPolicy::from_setting(None),
            LockWidgetPolicy::Summary
        );
        for value in [
            json!(null),
            json!(true),
            json!(42),
            json!({}),
            json!([]),
            json!("all"),
        ] {
            assert_eq!(
                LockWidgetPolicy::from_setting(Some(&value)),
                LockWidgetPolicy::Hidden
            );
        }
    }

    #[test]
    fn policies_round_trip_through_settings_json() {
        for policy in [
            LockWidgetPolicy::Hidden,
            LockWidgetPolicy::Device,
            LockWidgetPolicy::Summary,
        ] {
            let text = serde_json::to_string(&json!({"lock_widgets": policy.as_str()})).unwrap();
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(
                LockWidgetPolicy::from_setting(value.get("lock_widgets")),
                policy
            );
        }
    }

    #[test]
    fn complete_mode_and_lock_state_matrix() {
        for policy in [
            LockWidgetPolicy::Hidden,
            LockWidgetPolicy::Device,
            LockWidgetPolicy::Summary,
        ] {
            for sleeping in [false, true] {
                for pin in [false, true] {
                    for connected in [false, true] {
                        for attention in [false, true] {
                            let widgets = policy.project(
                                sleeping,
                                pin,
                                Some((87, true)),
                                connected,
                                attention,
                            );
                            if sleeping || pin || policy == LockWidgetPolicy::Hidden {
                                assert_eq!(widgets, LockWidgets::default());
                            } else {
                                assert_eq!(widgets.battery, Some((87, true)));
                                assert_eq!(
                                    widgets.attention,
                                    (policy == LockWidgetPolicy::Summary)
                                        .then_some((connected, attention))
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn absent_or_invalid_battery_is_not_invented() {
        for battery in [None, Some((101, false)), Some((255, true))] {
            assert_eq!(
                LockWidgetPolicy::Summary
                    .project(false, false, battery, true, false)
                    .battery,
                None
            );
        }
        assert_eq!(
            LockWidgetPolicy::Device
                .project(false, false, Some((0, false)), true, false)
                .battery,
            Some((0, false))
        );
    }
}
