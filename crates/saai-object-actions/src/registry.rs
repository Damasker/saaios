use crate::spec::{
    bind_arguments, ActionAvailability, ActionResolution, ActionResolveContext, ObjectActionError,
    ObjectActionSpec, ResolvedAction,
};
use saai_entity_store::{Entity, ObjectRef};
use std::collections::BTreeMap;
use tool_registry::ToolRegistry;

#[derive(Debug, Default)]
pub struct ObjectActionRegistry {
    specs: Vec<ObjectActionSpec>,
}

impl ObjectActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, spec: ObjectActionSpec) -> Result<(), ObjectActionError> {
        spec.validate()?;
        let key = spec.identity_key();
        if self
            .specs
            .iter()
            .any(|existing| existing.identity_key() == key)
        {
            return Err(ObjectActionError::DuplicateSpec);
        }
        self.specs.push(spec);
        Ok(())
    }

    pub fn list(&self) -> &[ObjectActionSpec] {
        &self.specs
    }

    /// Semantic action ids whose selector matches `entity_type`.
    /// Discovery only — does not probe providers or tools.
    pub fn action_ids_for_type(&self, entity_type: &str) -> Vec<String> {
        let mut ids: Vec<String> = self
            .specs
            .iter()
            .filter(|spec| spec.applies_to.matches(entity_type))
            .map(|spec| spec.action_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// The full spec behind a resolved `action_id` -- `ActionResolution`
    /// (`intent-resolution`'s own type) only carries the id string, so
    /// a caller that needs anything else about the action (today:
    /// `requires_confirmation`, so `saai-taskd` can decide whether it
    /// may auto-complete) looks it back up here. `None` is a real,
    /// checked case, not an invariant violation: the id came from a
    /// snapshot of this same registry, but nothing prevents it from
    /// changing between resolution and this lookup in a longer-lived
    /// process -- callers must treat it as "cannot confirm this is
    /// safe," never as "must be fine."
    pub fn spec_for(&self, action_id: &str) -> Option<&ObjectActionSpec> {
        self.specs.iter().find(|spec| spec.action_id == action_id)
    }

    pub fn resolve_for(
        &self,
        object: &Entity,
        context: &ActionResolveContext,
        tools: &ToolRegistry,
    ) -> Vec<ActionResolution> {
        let mut by_action: BTreeMap<String, Vec<ResolvedAction>> = BTreeMap::new();
        for spec in &self.specs {
            if !spec.applies_to.matches(&object.entity_type) {
                continue;
            }
            by_action
                .entry(spec.action_id.clone())
                .or_default()
                .push(resolve_spec(spec, object, context, tools));
        }
        by_action
            .into_iter()
            .flat_map(|(action_id, mut candidates)| {
                candidates.sort_by(|left, right| {
                    left.provider_id
                        .cmp(&right.provider_id)
                        .then_with(|| left.tool_name.cmp(&right.tool_name))
                });
                let available = candidates
                    .iter()
                    .filter(|candidate| candidate.availability.is_available())
                    .cloned()
                    .collect::<Vec<_>>();
                match available.len() {
                    0 => candidates
                        .into_iter()
                        .map(ActionResolution::Resolved)
                        .collect(),
                    1 => {
                        let mut available = available;
                        vec![ActionResolution::Resolved(available.remove(0))]
                    }
                    _ => vec![ActionResolution::Ambiguous {
                        action_id,
                        candidates: available,
                    }],
                }
            })
            .collect()
    }
}

fn resolve_spec(
    spec: &ObjectActionSpec,
    object: &Entity,
    context: &ActionResolveContext,
    tools: &ToolRegistry,
) -> ResolvedAction {
    let availability_and_args = if tools.get(&spec.tool_name).is_none() {
        (
            ActionAvailability::Unavailable {
                reason: "provider tool unavailable".into(),
            },
            serde_json::json!({}),
        )
    } else {
        match bind_arguments(spec, object, context) {
            Ok(arguments) => (ActionAvailability::Available, arguments),
            Err(reason) => (
                ActionAvailability::Unavailable { reason },
                serde_json::json!({}),
            ),
        }
    };
    ResolvedAction {
        action_id: spec.action_id.clone(),
        title: spec.title.clone(),
        description: spec.description.clone(),
        target: ObjectRef::entity(object.id),
        provider_id: spec.provider_id.clone(),
        tool_name: spec.tool_name.clone(),
        arguments: availability_and_args.1,
        availability: availability_and_args.0,
        object_revision: object.revision,
    }
}
