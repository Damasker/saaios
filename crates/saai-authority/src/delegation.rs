//! AUTH-06: a worker acts only through a bound DelegationEnvelope.
//! Not a second GrantStore. Persistent does not live here.

use crate::binding::canonical_json;
use crate::model::{
    request_operation_id, AuthorityRequest, GrantValidity, IdentityProof, Principal, PrincipalId,
    PrincipalKind,
};
use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegationEnvelope {
    pub issuer: PrincipalId,
    pub worker: PrincipalId,
    pub execution_id: Uuid,
    pub operation: String,
    pub target: Option<ObjectRef>,
    pub arguments: Value,
    pub validity: GrantValidity,
}

impl DelegationEnvelope {
    pub fn from_owner_request(
        request: &AuthorityRequest,
        worker: Principal,
        execution_id: Uuid,
        validity: GrantValidity,
    ) -> Option<Self> {
        if request.principal.id != PrincipalId::owner()
            || request.principal.kind != PrincipalKind::LocalUser
        {
            return None;
        }
        if worker.kind != PrincipalKind::Worker || worker.id != PrincipalId::worker(execution_id) {
            return None;
        }
        let operation = request_operation_id(request)?.to_string();
        Some(Self {
            issuer: request.principal.id.clone(),
            worker: worker.id,
            execution_id,
            operation,
            target: request.target.clone(),
            arguments: canonical_json(&request.arguments),
            validity,
        })
    }
}

pub fn envelope_covers(
    envelope: &DelegationEnvelope,
    request: &AuthorityRequest,
    now: DateTime<Utc>,
) -> bool {
    let IdentityProof::DelegatedWorker { execution_id } = request.proof else {
        return false;
    };
    grant_is_live_for(&envelope.validity, now)
        && execution_id == envelope.execution_id
        && request.principal.id == envelope.worker
        && request.principal.kind == PrincipalKind::Worker
        && request_operation_id(request) == Some(envelope.operation.as_str())
        && request.target == envelope.target
        && canonical_json(&request.arguments) == envelope.arguments
}

fn grant_is_live_for(validity: &GrantValidity, now: DateTime<Utc>) -> bool {
    match validity {
        GrantValidity::Session | GrantValidity::OneShot => true,
        GrantValidity::Until(until) => now < *until,
        GrantValidity::Persistent => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuthorityRequest;
    use chrono::Duration;
    use serde_json::json;

    fn owner_kill(target: ObjectRef, pid: u64) -> AuthorityRequest {
        AuthorityRequest::local_user_action("process.stop", Some(target), json!({"pid": pid}))
    }

    fn worker_kill(execution_id: Uuid, target: ObjectRef, pid: u64) -> AuthorityRequest {
        let mut request =
            AuthorityRequest::local_user_action("process.stop", Some(target), json!({"pid": pid}));
        request.principal = Principal::worker(execution_id);
        request.proof = IdentityProof::DelegatedWorker { execution_id };
        request
    }

    #[test]
    fn envelope_covers_the_bound_worker_request() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .expect("owner issues");
        let now = Utc::now();
        assert!(envelope_covers(
            &envelope,
            &worker_kill(execution_id, target, 4312),
            now
        ));
    }

    #[test]
    fn other_worker_is_not_covered() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .unwrap();
        let other = Uuid::new_v4();
        assert!(!envelope_covers(
            &envelope,
            &worker_kill(other, target, 4312),
            Utc::now()
        ));
    }

    #[test]
    fn changed_args_are_not_covered() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .unwrap();
        assert!(!envelope_covers(
            &envelope,
            &worker_kill(execution_id, target, 9999),
            Utc::now()
        ));
    }

    #[test]
    fn changed_target_is_not_covered() {
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(ObjectRef::entity(Uuid::new_v4()), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::Session,
        )
        .unwrap();
        assert!(!envelope_covers(
            &envelope,
            &worker_kill(execution_id, ObjectRef::entity(Uuid::new_v4()), 4312),
            Utc::now()
        ));
    }

    #[test]
    fn owner_request_is_not_an_envelope() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .unwrap();
        assert!(!envelope_covers(
            &envelope,
            &owner_kill(target, 4312),
            Utc::now()
        ));
    }

    #[test]
    fn worker_cannot_issue_as_owner() {
        let execution_id = Uuid::new_v4();
        let request = worker_kill(execution_id, ObjectRef::entity(Uuid::new_v4()), 4312);
        assert!(DelegationEnvelope::from_owner_request(
            &request,
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .is_none());
    }

    #[test]
    fn expired_until_is_not_live() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::Until(Utc::now() - Duration::seconds(1)),
        )
        .unwrap();
        assert!(!envelope_covers(
            &envelope,
            &worker_kill(execution_id, target, 4312),
            Utc::now()
        ));
    }

    #[test]
    fn persistent_envelope_is_not_live() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let envelope = DelegationEnvelope::from_owner_request(
            &owner_kill(target.clone(), 4312),
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::Persistent,
        )
        .unwrap();
        assert!(!grant_is_live_for(&envelope.validity, Utc::now()));
        assert!(!envelope_covers(
            &envelope,
            &worker_kill(execution_id, target, 4312),
            Utc::now()
        ));
    }

    #[test]
    fn argument_key_order_does_not_break_the_envelope() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let execution_id = Uuid::new_v4();
        let mut issued = AuthorityRequest::local_user_action(
            "process.stop",
            Some(target.clone()),
            json!({"note": "a", "pid": 4312}),
        );
        let envelope = DelegationEnvelope::from_owner_request(
            &issued,
            Principal::worker(execution_id),
            execution_id,
            GrantValidity::OneShot,
        )
        .unwrap();
        issued.arguments = json!({"pid": 4312, "note": "a"});
        let mut worker = issued;
        worker.principal = Principal::worker(execution_id);
        worker.proof = IdentityProof::DelegatedWorker { execution_id };
        assert!(envelope_covers(&envelope, &worker, Utc::now()));
    }
}
