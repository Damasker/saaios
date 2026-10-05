//! Intent Resolution & Action Bridge — maps captured context to a
//! semantic outcome. Does not execute tools, own policy, or replace
//! `saai-taskd`'s workflow.

mod context;
mod outcome;
mod resolve;

pub use context::{
    intent_properties, ContextSnapshot, IntentInput, IntentSource, ObjectSummary, CONTEXT_PROPERTY,
    SEMANTIC_ACTION_PROPERTY, SOURCE_PROPERTY, TEXT_PROPERTY,
};
pub use outcome::{
    ActionResolution, AnswerResolution, ClarificationOption, ClarificationResolution,
    PlanResolution, ResolutionMethod, ResolutionOutcome, ResolutionTrace, ResolveAttempt,
    UnsupportedResolution,
};
pub use resolve::{
    apply_clarification, resolve_deterministic, validate_action, ActionValidationError,
    AllowedContext,
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use saai_entity_store::{Entity, ObjectRef, SCHEMA_VERSION};
    use serde_json::{json, Map};
    use uuid::Uuid;

    fn entity(entity_type: &str, title: &str, revision: u64) -> Entity {
        let now = Utc::now();
        Entity {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(revision as u128 + 10),
            space_id: "work".into(),
            entity_type: entity_type.into(),
            title: title.into(),
            properties: Map::new(),
            revision,
            created_at: now,
            updated_at: now,
        }
    }

    fn display() -> Entity {
        entity("saaios.display", "Экран", 10)
    }

    fn summary(entity: &Entity) -> ObjectSummary {
        ObjectSummary::from_entity(entity)
    }

    fn input_for(entity: &Entity, text: &str) -> IntentInput {
        IntentInput {
            text: text.into(),
            source: IntentSource::Orb,
            explicit_action_id: None,
            primary_object: None,
            context: ContextSnapshot::from_focus("work", Some(entity)),
        }
    }

    fn allowed_for(entity: &Entity, actions: &[&str]) -> AllowedContext {
        AllowedContext::default().with_object(
            &summary(entity),
            actions.iter().map(|action| (*action).to_string()).collect(),
        )
    }

    fn action_of(attempt: ResolveAttempt) -> ActionResolution {
        match attempt {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Action(action),
                ..
            } => action,
            other => panic!("expected action, got {other:?}"),
        }
    }

    #[test]
    fn explicit_object_view_action_does_not_need_a_model() {
        let object = display();
        let mut input = input_for(&object, "");
        input.source = IntentSource::ObjectView;
        input.explicit_action_id = Some("display.inspect".into());
        input.primary_object = Some(ObjectRef::entity(object.id));
        let attempt = resolve_deterministic(&input, &allowed_for(&object, &["display.inspect"]));
        match attempt {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Action(action),
                trace,
            } => {
                assert_eq!(action.action_id, "display.inspect");
                assert_eq!(action.target, ObjectRef::entity(object.id));
                assert_eq!(trace.action_source, ResolutionMethod::ExplicitAction);
            }
            other => panic!("expected explicit action, got {other:?}"),
        }
    }

    #[test]
    fn focused_object_resolves_a_pronoun_inspect() {
        let object = display();
        let attempt = resolve_deterministic(
            &input_for(&object, "покажи его состояние"),
            &allowed_for(&object, &["display.inspect"]),
        );
        let action = action_of(attempt);
        assert_eq!(action.target, ObjectRef::entity(object.id));
        assert_eq!(action.action_id, "display.inspect");
        assert_eq!(action.target_revision, Some(10));
    }

    #[test]
    fn explicit_target_beats_current_focus() {
        let focused = display();
        let selected = entity("service.systemd", "prod-api", 4);
        let mut input = input_for(&focused, "перезапусти его");
        input.primary_object = Some(ObjectRef::entity(selected.id));
        input.context.selected_objects.push(summary(&selected));
        let allowed = allowed_for(&focused, &["display.inspect"])
            .with_object(&summary(&selected), vec!["service.restart".into()]);
        let action = action_of(resolve_deterministic(&input, &allowed));
        assert_eq!(action.target, ObjectRef::entity(selected.id));
        assert_eq!(action.action_id, "service.restart");
    }

    #[test]
    fn missing_target_produces_clarification() {
        let input = IntentInput {
            text: "перезапусти API".into(),
            source: IntentSource::Orb,
            explicit_action_id: None,
            primary_object: None,
            context: ContextSnapshot {
                primary_space_id: Some("work".into()),
                active_space_ids: vec!["work".into()],
                focused_object: None,
                selected_objects: Vec::new(),
            },
        };
        match resolve_deterministic(&input, &AllowedContext::default()) {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Clarification(clarification),
                ..
            } => assert!(clarification.question.contains("объект")),
            other => panic!("expected clarification, got {other:?}"),
        }
    }

    #[test]
    fn two_valid_targets_produce_clarification_not_a_guess() {
        let prod = entity("service.systemd", "Production API", 1);
        let stage = entity("service.systemd", "Stage API", 1);
        let input = IntentInput {
            text: "перезапусти API".into(),
            source: IntentSource::Orb,
            explicit_action_id: None,
            primary_object: None,
            context: ContextSnapshot {
                primary_space_id: Some("work".into()),
                active_space_ids: vec!["work".into()],
                focused_object: None,
                selected_objects: vec![summary(&prod), summary(&stage)],
            },
        };
        let allowed = AllowedContext::default()
            .with_object(&summary(&prod), vec!["service.restart".into()])
            .with_object(&summary(&stage), vec!["service.restart".into()]);
        match resolve_deterministic(&input, &allowed) {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Clarification(clarification),
                ..
            } => {
                assert_eq!(clarification.options.len(), 2);
            }
            other => panic!("expected clarification, got {other:?}"),
        }
    }

    #[test]
    fn model_cannot_return_an_unregistered_action_id() {
        let object = display();
        let proposed = ActionResolution {
            target: ObjectRef::entity(object.id),
            action_id: "service.reinstall-from-scratch".into(),
            parameters: json!({}),
            target_revision: Some(10),
        };
        assert_eq!(
            validate_action(&proposed, &allowed_for(&object, &["display.inspect"])),
            Err(ActionValidationError::UnregisteredAction)
        );
    }

    #[test]
    fn model_cannot_return_an_object_missing_from_context() {
        let visible = display();
        let invented = Uuid::from_u128(99);
        let proposed = ActionResolution {
            target: ObjectRef::entity(invented),
            action_id: "display.inspect".into(),
            parameters: json!({}),
            target_revision: None,
        };
        assert_eq!(
            validate_action(&proposed, &allowed_for(&visible, &["display.inspect"])),
            Err(ActionValidationError::UnknownObject)
        );
    }

    #[test]
    fn unsupported_action_becomes_unsupported() {
        let object = display();
        let mut input = input_for(&object, "");
        input.explicit_action_id = Some("server.reinstall".into());
        match resolve_deterministic(&input, &allowed_for(&object, &["display.inspect"])) {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Unsupported(_),
                ..
            } => {}
            other => panic!("expected unsupported, got {other:?}"),
        }
    }

    #[test]
    fn context_snapshot_is_immutable_after_intent_creation() {
        let object = display();
        let properties = intent_properties(
            "покажи его состояние",
            IntentSource::Orb,
            &ContextSnapshot::from_focus("work", Some(&object)),
            None,
        );
        let mut stored = object.clone();
        stored.properties = properties;
        stored.entity_type = "saaios.intent".into();
        let parsed = IntentInput::from_entity(&stored);
        let mut live_focus = object;
        live_focus.revision = 11;
        live_focus.id = Uuid::from_u128(77);
        let captured = parsed.context.focused_object.expect("captured focus");
        assert_eq!(captured.object, ObjectRef::entity(Uuid::from_u128(20)));
        assert_eq!(captured.revision, 10);
    }

    #[test]
    fn object_revision_is_preserved_on_action_resolution() {
        let object = display();
        let action = action_of(resolve_deterministic(
            &input_for(&object, "покажи его состояние"),
            &allowed_for(&object, &["display.inspect"]),
        ));
        assert_eq!(action.target_revision, Some(10));
        assert!(action.is_stale(11));
        assert!(!action.is_stale(10));
    }

    #[test]
    fn clarification_resumes_the_same_intent() {
        let prod = entity("service.systemd", "Production API", 1);
        let stage = entity("service.systemd", "Stage API", 2);
        let input = IntentInput {
            text: "перезапусти API".into(),
            source: IntentSource::Orb,
            explicit_action_id: None,
            primary_object: None,
            context: ContextSnapshot {
                primary_space_id: Some("work".into()),
                active_space_ids: vec!["work".into()],
                focused_object: None,
                selected_objects: vec![summary(&prod), summary(&stage)],
            },
        };
        let allowed = AllowedContext::default()
            .with_object(&summary(&prod), vec!["service.restart".into()])
            .with_object(&summary(&stage), vec!["service.restart".into()]);
        let ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Clarification(clarification),
            ..
        } = resolve_deterministic(&input, &allowed)
        else {
            panic!("expected clarification");
        };
        let stage_option = clarification
            .options
            .iter()
            .find(|option| option.label == "Stage API")
            .unwrap();
        let resumed = apply_clarification(&input, stage_option);
        assert_eq!(resumed.primary_object, Some(ObjectRef::entity(stage.id)));
        let action = action_of(resolve_deterministic(&resumed, &allowed));
        assert_eq!(action.target, ObjectRef::entity(stage.id));
        assert_eq!(action.action_id, "service.restart");
    }

    #[test]
    fn confirmation_is_not_a_clarification_outcome() {
        let object = display();
        match resolve_deterministic(
            &input_for(&object, "покажи его состояние"),
            &allowed_for(&object, &["display.inspect"]),
        ) {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Action(_),
                ..
            } => {}
            other => panic!("inspect is an action, not clarification: {other:?}"),
        }
    }

    #[test]
    fn free_form_without_context_still_needs_the_model() {
        let input = IntentInput {
            text: "сколько места осталось?".into(),
            source: IntentSource::Orb,
            explicit_action_id: None,
            primary_object: None,
            context: ContextSnapshot::default(),
        };
        assert!(matches!(
            resolve_deterministic(&input, &AllowedContext::default()),
            ResolveAttempt::NeedsModel
        ));
    }
}
