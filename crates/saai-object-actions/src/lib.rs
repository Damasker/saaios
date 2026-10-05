//! Object Action Model — semantic actions over SOM objects.
//!
//! Does not replace ToolRegistry, PolicyEngine, app capabilities, or
//! the Intent/Task/Action workflow. It only answers which verbs apply
//! to a given object and how to bind them to an existing tool.

mod execute;
mod registry;
mod spec;

pub use execute::{execute_if_allowed, preflight, ExecuteError};
pub use registry::ObjectActionRegistry;
pub use spec::{
    bind_arguments, display_inspect_spec, ActionAvailability, ActionResolution,
    ActionResolveContext, ArgumentBinding, ArgumentSource, ObjectActionError, ObjectActionSpec,
    ObjectSelector, ResolvedAction,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use policy_engine::PolicyEngine;
    use protocol::PolicyVerdict;
    use saai_entity_store::{Entity, SCHEMA_VERSION};
    use serde_json::{json, Map};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tool_registry::{
        RiskLevel, ToolContext, ToolError, ToolExecutor, ToolOutput, ToolRegistry, ToolSpec,
    };
    use uuid::Uuid;

    struct CountingTool {
        spec: ToolSpec,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolExecutor for CountingTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }

        async fn execute(
            &self,
            args: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolOutput, ToolError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ToolOutput {
                ok: true,
                value: args,
                error: None,
            })
        }
    }

    fn spec(name: &str, risk: RiskLevel, confirm: bool) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: name.into(),
            risk,
            timeout_ms: 100,
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: confirm,
        }
    }

    fn counting(
        name: &str,
        risk: RiskLevel,
        confirm: bool,
    ) -> (Arc<CountingTool>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let tool = Arc::new(CountingTool {
            spec: spec(name, risk, confirm),
            calls: calls.clone(),
        });
        (tool, calls)
    }

    fn object(entity_type: &str, properties: Map<String, serde_json::Value>) -> Entity {
        let now = Utc::now();
        Entity {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(42),
            space_id: "saaios".into(),
            entity_type: entity_type.into(),
            title: "Экран".into(),
            properties,
            revision: 4,
            created_at: now,
            updated_at: now,
        }
    }

    fn action_spec(
        action_id: &str,
        provider: &str,
        entity_type: &str,
        tool_name: &str,
        arguments: Vec<ArgumentBinding>,
        required: Vec<String>,
    ) -> ObjectActionSpec {
        ObjectActionSpec {
            action_id: action_id.into(),
            title: "Inspect".into(),
            description: "Read-only inspect".into(),
            applies_to: ObjectSelector {
                entity_types: vec![entity_type.into()],
            },
            tool_name: tool_name.into(),
            arguments,
            required_properties: required,
            provider_id: provider.into(),
        }
    }

    fn resolved(
        registry: &ObjectActionRegistry,
        object: &Entity,
        tools: &ToolRegistry,
    ) -> Vec<ActionResolution> {
        registry.resolve_for(object, &ActionResolveContext::default(), tools)
    }

    fn only_resolved(resolutions: Vec<ActionResolution>) -> ResolvedAction {
        match resolutions.as_slice() {
            [ActionResolution::Resolved(action)] => action.clone(),
            other => panic!("expected one resolved action, got {other:?}"),
        }
    }

    fn ctx() -> ToolContext {
        ToolContext {
            correlation_id: Uuid::from_u128(1),
            call_id: Uuid::from_u128(2),
            space_id: Some("saaios".into()),
        }
    }

    #[test]
    fn valid_spec_registers() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        assert_eq!(registry.list().len(), 1);
        assert_eq!(
            registry.action_ids_for_type("saaios.display"),
            ["display.inspect"]
        );
    }

    #[test]
    fn invalid_action_id_is_rejected() {
        let mut spec = display_inspect_spec();
        spec.action_id = "Display.Inspect".into();
        assert_eq!(spec.validate(), Err(ObjectActionError::InvalidActionId));
        spec = display_inspect_spec();
        spec.action_id = "process.kill_request".into();
        assert_eq!(spec.validate(), Err(ObjectActionError::InvalidActionId));
    }

    #[test]
    fn missing_tool_is_unavailable() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let action = only_resolved(resolved(
            &registry,
            &object("saaios.display", Map::new()),
            &ToolRegistry::new(),
        ));
        assert_eq!(
            action.availability,
            ActionAvailability::Unavailable {
                reason: "provider tool unavailable".into()
            }
        );
    }

    #[test]
    fn wrong_entity_type_does_not_match() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let (tool, _) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        assert!(resolved(&registry, &object("document.file", Map::new()), &tools).is_empty());
    }

    #[test]
    fn matching_entity_type_resolves() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let (tool, _) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let action = only_resolved(resolved(
            &registry,
            &object("saaios.display", Map::new()),
            &tools,
        ));
        assert_eq!(action.action_id, "display.inspect");
        assert_eq!(action.tool_name, "system.identity");
        assert_eq!(action.availability, ActionAvailability::Available);
    }

    #[test]
    fn missing_required_property_is_unavailable() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "service.restart",
                "saai.systemd",
                "service.systemd",
                "system.identity",
                vec![ArgumentBinding {
                    argument: "host".into(),
                    source: ArgumentSource::ObjectProperty {
                        name: "hostname".into(),
                    },
                }],
                vec!["hostname".into()],
            ))
            .unwrap();
        let (tool, _) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let action = only_resolved(resolved(
            &registry,
            &object("service.systemd", Map::new()),
            &tools,
        ));
        assert!(matches!(
            action.availability,
            ActionAvailability::Unavailable { reason } if reason == "missing object property: hostname"
        ));
    }

    #[test]
    fn property_constant_id_and_space_bindings_work() {
        let spec = action_spec(
            "service.inspect",
            "saai.systemd",
            "service.systemd",
            "system.identity",
            vec![
                ArgumentBinding {
                    argument: "kind".into(),
                    source: ArgumentSource::Constant(json!("systemd")),
                },
                ArgumentBinding {
                    argument: "id".into(),
                    source: ArgumentSource::ObjectId,
                },
                ArgumentBinding {
                    argument: "title".into(),
                    source: ArgumentSource::ObjectTitle,
                },
                ArgumentBinding {
                    argument: "host".into(),
                    source: ArgumentSource::ObjectProperty {
                        name: "hostname".into(),
                    },
                },
                ArgumentBinding {
                    argument: "space".into(),
                    source: ArgumentSource::ContextSpaceId,
                },
            ],
            vec![],
        );
        let mut properties = Map::new();
        properties.insert("hostname".into(), json!("prod-api-01"));
        let object = object("service.systemd", properties);
        let bound = bind_arguments(
            &spec,
            &object,
            &ActionResolveContext {
                space_id: Some("work".into()),
            },
        )
        .unwrap();
        assert_eq!(
            bound,
            json!({
                "kind": "systemd",
                "id": object.id.to_string(),
                "title": "Экран",
                "host": "prod-api-01",
                "space": "work"
            })
        );
    }

    #[test]
    fn duplicate_exact_spec_is_rejected() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        assert_eq!(
            registry.register(display_inspect_spec()),
            Err(ObjectActionError::DuplicateSpec)
        );
    }

    #[test]
    fn single_provider_resolves_deterministically() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let (tool, _) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let first = resolved(&registry, &object("saaios.display", Map::new()), &tools);
        let second = resolved(&registry, &object("saaios.display", Map::new()), &tools);
        assert_eq!(first, second);
        assert!(matches!(first[0], ActionResolution::Resolved(_)));
    }

    #[test]
    fn two_matching_providers_are_ambiguous_not_first_registered() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "service.restart",
                "saai.docker",
                "service.container",
                "system.identity",
                vec![],
                vec![],
            ))
            .unwrap();
        registry
            .register(action_spec(
                "service.restart",
                "saai.systemd",
                "service.container",
                "system.metrics",
                vec![],
                vec![],
            ))
            .unwrap();
        let (identity, _) = counting("system.identity", RiskLevel::Low, false);
        let (metrics, _) = counting("system.metrics", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(identity);
        tools.register(metrics);
        match resolved(&registry, &object("service.container", Map::new()), &tools).as_slice() {
            [ActionResolution::Ambiguous {
                action_id,
                candidates,
            }] => {
                assert_eq!(action_id, "service.restart");
                assert_eq!(
                    candidates
                        .iter()
                        .map(|candidate| candidate.provider_id.as_str())
                        .collect::<Vec<_>>(),
                    ["saai.docker", "saai.systemd"]
                );
            }
            other => panic!("expected ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn available_action_can_still_be_policy_deny() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "storage.wipe",
                "saai.local-system",
                "saaios.display",
                "storage.format",
                vec![],
                vec![],
            ))
            .unwrap();
        let (tool, calls) = counting("storage.format", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let action = only_resolved(resolved(
            &registry,
            &object("saaios.display", Map::new()),
            &tools,
        ));
        assert_eq!(action.availability, ActionAvailability::Available);
        let policy = PolicyEngine::new();
        assert_eq!(
            preflight(&policy, &tools, &action).verdict,
            PolicyVerdict::Deny
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn available_action_can_ask_user() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "process.stop",
                "saai.local-system",
                "saaios.display",
                "process.kill_request",
                vec![ArgumentBinding {
                    argument: "pid".into(),
                    source: ArgumentSource::Constant(json!(4312)),
                }],
                vec![],
            ))
            .unwrap();
        let (tool, _) = counting("process.kill_request", RiskLevel::High, true);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let action = only_resolved(resolved(
            &registry,
            &object("saaios.display", Map::new()),
            &tools,
        ));
        let policy = PolicyEngine::new();
        assert_eq!(
            preflight(&policy, &tools, &action).verdict,
            PolicyVerdict::AskUser
        );
    }

    #[test]
    fn session_grant_is_read_from_the_live_engine() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "process.stop",
                "saai.local-system",
                "saaios.display",
                "process.kill_request",
                vec![ArgumentBinding {
                    argument: "pid".into(),
                    source: ArgumentSource::Constant(json!(4312)),
                }],
                vec![],
            ))
            .unwrap();
        let (tool, _) = counting("process.kill_request", RiskLevel::High, true);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let action = only_resolved(resolved(
            &registry,
            &object("saaios.display", Map::new()),
            &tools,
        ));
        let policy = PolicyEngine::new();
        assert_eq!(
            preflight(&policy, &tools, &action).verdict,
            PolicyVerdict::AskUser
        );
        policy.grant_session("process.kill_request");
        assert_eq!(
            preflight(&policy, &tools, &action).verdict,
            PolicyVerdict::Allow
        );
    }

    #[tokio::test]
    async fn deny_does_not_call_the_executor() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "storage.wipe",
                "saai.local-system",
                "saaios.display",
                "storage.format",
                vec![],
                vec![],
            ))
            .unwrap();
        let (tool, calls) = counting("storage.format", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let object = object("saaios.display", Map::new());
        let action = only_resolved(resolved(&registry, &object, &tools));
        let policy = PolicyEngine::new();
        let error = execute_if_allowed(&policy, &tools, &action, &object, &ctx())
            .await
            .unwrap_err();
        assert!(matches!(error, ExecuteError::Denied(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn ask_user_does_not_call_the_executor() {
        let mut registry = ObjectActionRegistry::new();
        registry
            .register(action_spec(
                "process.stop",
                "saai.local-system",
                "saaios.display",
                "process.kill_request",
                vec![ArgumentBinding {
                    argument: "pid".into(),
                    source: ArgumentSource::Constant(json!(4312)),
                }],
                vec![],
            ))
            .unwrap();
        let (tool, calls) = counting("process.kill_request", RiskLevel::High, true);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let object = object("saaios.display", Map::new());
        let action = only_resolved(resolved(&registry, &object, &tools));
        let policy = PolicyEngine::new();
        let error = execute_if_allowed(&policy, &tools, &action, &object, &ctx())
            .await
            .unwrap_err();
        assert!(matches!(error, ExecuteError::NeedsConfirmation(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn allow_executes_and_policy_is_rechecked() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let (tool, calls) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let object = object("saaios.display", Map::new());
        let action = only_resolved(resolved(&registry, &object, &tools));
        let policy = PolicyEngine::new();
        assert_eq!(
            preflight(&policy, &tools, &action).verdict,
            PolicyVerdict::Allow
        );
        execute_if_allowed(&policy, &tools, &action, &object, &ctx())
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stale_object_revision_is_rejected() {
        let mut registry = ObjectActionRegistry::new();
        registry.register(display_inspect_spec()).unwrap();
        let (tool, calls) = counting("system.identity", RiskLevel::Low, false);
        let mut tools = ToolRegistry::new();
        tools.register(tool);
        let mut current = object("saaios.display", Map::new());
        let action = only_resolved(resolved(&registry, &current, &tools));
        current.revision = 5;
        let policy = PolicyEngine::new();
        let error = execute_if_allowed(&policy, &tools, &action, &current, &ctx())
            .await
            .unwrap_err();
        assert!(matches!(error, ExecuteError::StaleObject));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
