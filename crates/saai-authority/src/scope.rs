//! Scope matching is pure: no I/O, no model.

use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetScope {
    Any,
    ExactObject { object: ObjectRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SpaceScope {
    Any,
    ExactSpace { id: String },
}

pub fn scope_matches(scope: &TargetScope, target: Option<&ObjectRef>) -> bool {
    match (scope, target) {
        (TargetScope::Any, _) => true,
        (TargetScope::ExactObject { object }, Some(got)) => object == got,
        (TargetScope::ExactObject { .. }, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn exact_object_matches_same_ref() {
        let id = Uuid::new_v4();
        let object = ObjectRef::entity(id);
        let scope = TargetScope::ExactObject {
            object: object.clone(),
        };
        assert!(scope_matches(&scope, Some(&object)));
        assert!(!scope_matches(
            &scope,
            Some(&ObjectRef::entity(Uuid::new_v4()))
        ));
        assert!(!scope_matches(&scope, None));
    }

    #[test]
    fn any_matches_missing_target() {
        assert!(scope_matches(&TargetScope::Any, None));
    }
}
