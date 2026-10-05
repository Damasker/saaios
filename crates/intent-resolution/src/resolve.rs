use crate::context::{ContextSnapshot, IntentInput, ObjectSummary};
use crate::outcome::{
    ActionResolution, ClarificationOption, ClarificationResolution, ResolutionMethod,
    ResolutionOutcome, ResolutionTrace, ResolveAttempt, UnsupportedResolution,
};
use saai_entity_store::ObjectRef;
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct AllowedContext {
    pub visible_objects: HashSet<ObjectRef>,
    pub actions_for_target: HashMap<ObjectRef, Vec<String>>,
}

impl AllowedContext {
    pub fn from_snapshot(snapshot: &ContextSnapshot, actions: &[String]) -> Self {
        let mut visible_objects = HashSet::new();
        let mut actions_for_target = HashMap::new();
        for summary in snapshot
            .focused_object
            .iter()
            .chain(snapshot.selected_objects.iter())
        {
            visible_objects.insert(summary.object.clone());
            actions_for_target.insert(summary.object.clone(), actions.to_vec());
        }
        Self {
            visible_objects,
            actions_for_target,
        }
    }

    pub fn with_object(mut self, summary: &ObjectSummary, actions: Vec<String>) -> Self {
        self.visible_objects.insert(summary.object.clone());
        self.actions_for_target
            .insert(summary.object.clone(), actions);
        self
    }

    fn actions_for(&self, target: &ObjectRef) -> &[String] {
        self.actions_for_target
            .get(target)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn contains_object(&self, target: &ObjectRef) -> bool {
        self.visible_objects.contains(target)
    }

    pub fn allows_action(&self, target: &ObjectRef, action_id: &str) -> bool {
        self.actions_for(target).iter().any(|id| id == action_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionValidationError {
    UnknownObject,
    UnregisteredAction,
}

pub fn validate_action(
    resolution: &ActionResolution,
    allowed: &AllowedContext,
) -> Result<(), ActionValidationError> {
    if !allowed.contains_object(&resolution.target) {
        return Err(ActionValidationError::UnknownObject);
    }
    if !allowed.allows_action(&resolution.target, &resolution.action_id) {
        return Err(ActionValidationError::UnregisteredAction);
    }
    Ok(())
}

/// Deterministic IRAB path. Never calls a model. Never executes.
pub fn resolve_deterministic(input: &IntentInput, allowed: &AllowedContext) -> ResolveAttempt {
    if let Some(action_id) = input.explicit_action_id.as_deref() {
        return resolve_explicit(input, allowed, action_id);
    }
    if let Some(attempt) = resolve_pronoun_inspect(input, allowed) {
        return attempt;
    }
    if let Some(attempt) = resolve_targeted_verb(input, allowed) {
        return attempt;
    }
    if let Some(attempt) = resolve_ambiguous_action(input, allowed) {
        return attempt;
    }
    ResolveAttempt::NeedsModel
}

fn resolve_explicit(
    input: &IntentInput,
    allowed: &AllowedContext,
    action_id: &str,
) -> ResolveAttempt {
    let Some(target) = input
        .primary_object
        .clone()
        .or_else(|| input.context.focused_ref().cloned())
    else {
        return ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Clarification(ClarificationResolution {
                question: "С каким объектом выполнить действие?".into(),
                options: Vec::new(),
            }),
            trace: trace(
                ResolutionMethod::ExplicitAction,
                ResolutionMethod::ExplicitAction,
            ),
        };
    };
    let revision = revision_for(&input.context, &target);
    let resolution = ActionResolution {
        target,
        action_id: action_id.to_string(),
        parameters: json!({}),
        target_revision: revision,
    };
    match validate_action(&resolution, allowed) {
        Ok(()) => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Action(resolution),
            trace: trace(
                ResolutionMethod::ExplicitAction,
                ResolutionMethod::ExplicitAction,
            ),
        },
        Err(ActionValidationError::UnknownObject) => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Unsupported(UnsupportedResolution {
                reason: "target was not in the captured context".into(),
            }),
            trace: trace(
                ResolutionMethod::ExplicitAction,
                ResolutionMethod::ExplicitAction,
            ),
        },
        Err(ActionValidationError::UnregisteredAction) => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Unsupported(UnsupportedResolution {
                reason: format!("action `{action_id}` is not available for the target"),
            }),
            trace: trace(
                ResolutionMethod::ExplicitAction,
                ResolutionMethod::ExplicitAction,
            ),
        },
    }
}

fn resolve_pronoun_inspect(
    input: &IntentInput,
    allowed: &AllowedContext,
) -> Option<ResolveAttempt> {
    if !looks_like_pronoun_inspect(&input.text) {
        return None;
    }
    let focused = input.context.focused_object.as_ref()?;
    let inspect = inspect_actions(allowed.actions_for(&focused.object));
    Some(match inspect.as_slice() {
        [] => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Unsupported(UnsupportedResolution {
                reason: "no inspect action is registered for the focused object".into(),
            }),
            trace: trace(
                ResolutionMethod::FocusedObject,
                ResolutionMethod::DeterministicRule,
            ),
        },
        [action_id] => {
            let resolution = ActionResolution {
                target: focused.object.clone(),
                action_id: action_id.clone(),
                parameters: json!({}),
                target_revision: Some(focused.revision),
            };
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Action(resolution),
                trace: trace(
                    ResolutionMethod::FocusedObject,
                    ResolutionMethod::DeterministicRule,
                ),
            }
        }
        _ => ResolveAttempt::Resolved {
            outcome: clarification_for(
                "Какое состояние показать?",
                inspect.into_iter().map(|action_id| ClarificationOption {
                    id: action_id.clone(),
                    label: action_id,
                    object_ref: Some(focused.object.clone()),
                    action_id: None,
                }),
            ),
            trace: trace(
                ResolutionMethod::FocusedObject,
                ResolutionMethod::DeterministicRule,
            ),
        },
    })
}

fn resolve_targeted_verb(input: &IntentInput, allowed: &AllowedContext) -> Option<ResolveAttempt> {
    if !looks_like_underspecified_action(&input.text) {
        return None;
    }
    let target = input
        .primary_object
        .clone()
        .or_else(|| input.context.focused_ref().cloned())?;
    let restart = restart_actions(allowed.actions_for(&target));
    Some(match restart.as_slice() {
        [] => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Unsupported(UnsupportedResolution {
                reason: "no matching action is registered for the target".into(),
            }),
            trace: trace(
                ResolutionMethod::FocusedObject,
                ResolutionMethod::DeterministicRule,
            ),
        },
        [action_id] => ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Action(ActionResolution {
                target: target.clone(),
                action_id: action_id.clone(),
                parameters: json!({}),
                target_revision: revision_for(&input.context, &target),
            }),
            trace: trace(
                ResolutionMethod::FocusedObject,
                ResolutionMethod::DeterministicRule,
            ),
        },
        _ => ResolveAttempt::NeedsModel,
    })
}

fn resolve_ambiguous_action(
    input: &IntentInput,
    _allowed: &AllowedContext,
) -> Option<ResolveAttempt> {
    if input.context.focused_object.is_some() || input.primary_object.is_some() {
        return None;
    }
    if !looks_like_underspecified_action(&input.text) {
        return None;
    }
    let candidates: Vec<&ObjectSummary> = input.context.selected_objects.iter().collect();
    if candidates.len() < 2 {
        return Some(ResolveAttempt::Resolved {
            outcome: ResolutionOutcome::Clarification(ClarificationResolution {
                question: "Какой объект имеется в виду?".into(),
                options: Vec::new(),
            }),
            trace: trace(
                ResolutionMethod::DeterministicRule,
                ResolutionMethod::DeterministicRule,
            ),
        });
    }
    Some(ResolveAttempt::Resolved {
        outcome: clarification_for(
            "Какой объект?",
            candidates.into_iter().map(|summary| ClarificationOption {
                id: match &summary.object {
                    ObjectRef::Entity { id } => id.to_string(),
                    ObjectRef::Space { id } => id.clone(),
                },
                label: summary.title.clone(),
                object_ref: Some(summary.object.clone()),
                action_id: None,
            }),
        ),
        trace: trace(
            ResolutionMethod::DeterministicRule,
            ResolutionMethod::DeterministicRule,
        ),
    })
}

fn clarification_for(
    question: &str,
    options: impl IntoIterator<Item = ClarificationOption>,
) -> ResolutionOutcome {
    ResolutionOutcome::Clarification(ClarificationResolution {
        question: question.into(),
        options: options.into_iter().collect(),
    })
}

fn inspect_actions(actions: &[String]) -> Vec<String> {
    matching_actions(actions, &[".inspect", ".status", ".logs"])
}

fn restart_actions(actions: &[String]) -> Vec<String> {
    matching_actions(actions, &[".restart", ".stop", ".delete"])
}

fn matching_actions(actions: &[String], suffixes: &[&str]) -> Vec<String> {
    let mut matched: Vec<String> = actions
        .iter()
        .filter(|action_id| suffixes.iter().any(|suffix| action_id.ends_with(suffix)))
        .cloned()
        .collect();
    matched.sort();
    matched.dedup();
    matched
}

fn looks_like_pronoun_inspect(text: &str) -> bool {
    let lowered = text.to_lowercase();
    let mentions_pronoun = ["его", "это", "ним", "нём", " it", "this", "that"]
        .iter()
        .any(|token| lowered.contains(token));
    let mentions_status = [
        "состояни",
        "статус",
        "inspect",
        "покажи",
        "show",
        "status",
        "состояние",
    ]
    .iter()
    .any(|token| lowered.contains(token));
    mentions_pronoun && mentions_status
}

fn looks_like_underspecified_action(text: &str) -> bool {
    let lowered = text.to_lowercase();
    ["перезапуст", "restart", "удали", "delete"]
        .iter()
        .any(|token| lowered.contains(token))
}

fn revision_for(context: &ContextSnapshot, target: &ObjectRef) -> Option<u64> {
    context
        .focused_object
        .iter()
        .chain(context.selected_objects.iter())
        .find(|summary| &summary.object == target)
        .map(|summary| summary.revision)
}

fn trace(target_source: ResolutionMethod, action_source: ResolutionMethod) -> ResolutionTrace {
    ResolutionTrace {
        target_source,
        action_source,
        clarification_count: 0,
    }
}

/// Continue the same Intent after the user picks a clarification option.
pub fn apply_clarification(input: &IntentInput, option: &ClarificationOption) -> IntentInput {
    let mut next = input.clone();
    if let Some(object_ref) = &option.object_ref {
        next.primary_object = Some(object_ref.clone());
        if let Some(summary) = next
            .context
            .selected_objects
            .iter()
            .find(|candidate| &candidate.object == object_ref)
            .cloned()
        {
            next.context.focused_object = Some(summary);
        }
    }
    if let Some(action_id) = &option.action_id {
        next.explicit_action_id = Some(action_id.clone());
    }
    next
}
