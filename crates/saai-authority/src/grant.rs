//! AUTH-03: a session grant is Principal + operation + scope + validity.
//! Not a tool-name HashSet. Persistent grants do not live here.

use crate::model::{GrantValidity, PrincipalId};
use crate::scope::{scope_matches, SpaceScope, TargetScope};
use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGrant {
    pub principal: PrincipalId,
    pub operation: String,
    pub target: TargetScope,
    pub space: SpaceScope,
    pub validity: GrantValidity,
}

impl SessionGrant {
    pub fn session_any(principal: PrincipalId, operation: impl Into<String>) -> Self {
        Self {
            principal,
            operation: operation.into(),
            target: TargetScope::Any,
            space: SpaceScope::Any,
            validity: GrantValidity::Session,
        }
    }
}

pub fn grant_is_live(grant: &SessionGrant, now: DateTime<Utc>) -> bool {
    match grant.validity {
        GrantValidity::Session | GrantValidity::OneShot => true,
        GrantValidity::Until(until) => now < until,
        GrantValidity::Persistent => false,
    }
}

pub fn space_scope_matches(scope: &SpaceScope, space_ids: &[String]) -> bool {
    match scope {
        SpaceScope::Any => true,
        SpaceScope::ExactSpace { id } => space_ids.iter().any(|got| got == id),
    }
}

pub fn grant_covers(
    grant: &SessionGrant,
    principal: &PrincipalId,
    operation: &str,
    target: Option<&ObjectRef>,
    space_ids: &[String],
    now: DateTime<Utc>,
) -> bool {
    grant.principal == *principal
        && grant.operation == operation
        && grant_is_live(grant, now)
        && scope_matches(&grant.target, target)
        && space_scope_matches(&grant.space, space_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PrincipalId;
    use chrono::Duration;
    use saai_entity_store::ObjectRef;
    use uuid::Uuid;

    #[test]
    fn owner_grant_does_not_cover_worker() {
        let grant = SessionGrant::session_any(PrincipalId::owner(), "process.kill_request");
        let now = Utc::now();
        assert!(grant_covers(
            &grant,
            &PrincipalId::owner(),
            "process.kill_request",
            None,
            &[],
            now
        ));
        assert!(!grant_covers(
            &grant,
            &PrincipalId::worker(Uuid::new_v4()),
            "process.kill_request",
            None,
            &[],
            now
        ));
    }

    #[test]
    fn exact_object_grant_does_not_cover_other_object() {
        let object = ObjectRef::entity(Uuid::new_v4());
        let grant = SessionGrant {
            principal: PrincipalId::owner(),
            operation: "display.set_brightness".into(),
            target: TargetScope::ExactObject {
                object: object.clone(),
            },
            space: SpaceScope::Any,
            validity: GrantValidity::Session,
        };
        let now = Utc::now();
        assert!(grant_covers(
            &grant,
            &PrincipalId::owner(),
            "display.set_brightness",
            Some(&object),
            &[],
            now
        ));
        assert!(!grant_covers(
            &grant,
            &PrincipalId::owner(),
            "display.set_brightness",
            Some(&ObjectRef::entity(Uuid::new_v4())),
            &[],
            now
        ));
        assert!(!grant_covers(
            &grant,
            &PrincipalId::owner(),
            "display.set_brightness",
            None,
            &[],
            now
        ));
    }

    #[test]
    fn exact_space_grant_does_not_cover_other_space() {
        let grant = SessionGrant {
            principal: PrincipalId::owner(),
            operation: "entity.read".into(),
            target: TargetScope::Any,
            space: SpaceScope::ExactSpace { id: "home".into() },
            validity: GrantValidity::Session,
        };
        let now = Utc::now();
        assert!(grant_covers(
            &grant,
            &PrincipalId::owner(),
            "entity.read",
            None,
            &["home".into()],
            now
        ));
        assert!(!grant_covers(
            &grant,
            &PrincipalId::owner(),
            "entity.read",
            None,
            &["work".into()],
            now
        ));
    }

    #[test]
    fn expired_until_is_not_live() {
        let grant = SessionGrant {
            principal: PrincipalId::owner(),
            operation: "process.kill_request".into(),
            target: TargetScope::Any,
            space: SpaceScope::Any,
            validity: GrantValidity::Until(Utc::now() - Duration::seconds(1)),
        };
        assert!(!grant_is_live(&grant, Utc::now()));
        assert!(!grant_covers(
            &grant,
            &PrincipalId::owner(),
            "process.kill_request",
            None,
            &[],
            Utc::now()
        ));
    }

    #[test]
    fn persistent_is_not_a_session_grant() {
        let grant = SessionGrant {
            principal: PrincipalId::owner(),
            operation: "process.kill_request".into(),
            target: TargetScope::Any,
            space: SpaceScope::Any,
            validity: GrantValidity::Persistent,
        };
        assert!(!grant_is_live(&grant, Utc::now()));
    }
}
