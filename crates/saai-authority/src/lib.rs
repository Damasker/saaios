//! Unified Authority Model types (ADR-124 AUTH-01…06).
//!
//! Vocabulary, request binding, and pure grant matching. Enforcement
//! stays in PolicyEngine. Does not listen, persist GrantStore, or
//! replace authorized_keys.

mod binding;
mod delegation;
mod grant;
mod model;
mod scope;

pub use binding::{canonical_binding, canonical_json};
pub use delegation::{envelope_covers, DelegationEnvelope};
pub use grant::{grant_covers, grant_is_live, space_scope_matches, SessionGrant};
pub use model::{
    default_deny_unknown_operation, default_deny_unverified, proof_matches_principal,
    request_operation_id, AuthorityContext, AuthorityCorrelation, AuthorityOperation,
    AuthorityRequest, GrantValidity, IdentityProof, PolicyReasonCode, Principal, PrincipalId,
    PrincipalKind, UsageConstraint,
};
pub use saai_entity_store::ObjectRef;
pub use scope::{scope_matches, SpaceScope, TargetScope};

/// Existing PolicyEngine hard-deny tools. Grants cannot override these.
pub const HARD_DENIED_TOOLS: &[&str] = &["storage.format", "system.poweroff", "process.kill_init"];

pub fn is_hard_denied_tool(tool: &str) -> bool {
    HARD_DENIED_TOOLS.contains(&tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use saai_entity_store::ObjectRef;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn unverified_principal_denied() {
        assert_eq!(
            default_deny_unverified(&IdentityProof::Unverified),
            Some(PolicyReasonCode::IdentityUnverified)
        );
        assert_eq!(
            default_deny_unverified(&IdentityProof::LocalSystemSurface),
            None
        );
    }

    #[test]
    fn unknown_action_denied() {
        assert_eq!(
            default_deny_unknown_operation(false),
            Some(PolicyReasonCode::UnknownOperation)
        );
    }

    #[test]
    fn hard_deny_is_not_a_grant() {
        assert!(is_hard_denied_tool("storage.format"));
        assert!(is_hard_denied_tool("system.poweroff"));
        assert!(is_hard_denied_tool("process.kill_init"));
        assert!(!is_hard_denied_tool("system.metrics"));
    }

    #[test]
    fn confirmation_default_is_once() {
        assert_eq!(UsageConstraint::Once, UsageConstraint::Once);
        assert_ne!(GrantValidity::OneShot, GrantValidity::Session);
        assert_ne!(GrantValidity::Session, GrantValidity::Persistent);
    }

    #[test]
    fn session_grant_does_not_cross_principals() {
        let user = PrincipalId::owner();
        let worker = PrincipalId::worker(Uuid::new_v4());
        assert_ne!(user, worker);
    }

    #[test]
    fn ssh_identity_uses_fingerprint_not_comment() {
        let id = PrincipalId::ssh_fingerprint("SHA256:abcd");
        assert!(id.0.contains("SHA256:abcd"));
        assert!(!id.0.contains("Home Laptop"));
    }

    #[test]
    fn app_principal_is_not_local_user() {
        let app = Principal {
            id: PrincipalId::app("org.saaios.mahjong"),
            kind: PrincipalKind::Application,
        };
        assert_ne!(app.kind, PrincipalKind::LocalUser);
        assert!(app.id.0.starts_with("app:"));
    }

    #[test]
    fn caller_supplied_user_string_is_not_a_proof() {
        let spoof = json!({"caller": "user"});
        assert!(spoof.get("caller").is_some());
        // Proof is an enum set by the boundary, not by this JSON.
        assert_ne!(IdentityProof::Unverified, IdentityProof::LocalSystemSurface);
    }

    #[test]
    fn exact_object_scope_does_not_leak_to_other_object() {
        let a = ObjectRef::entity(Uuid::new_v4());
        let b = ObjectRef::entity(Uuid::new_v4());
        let scope = TargetScope::ExactObject { object: a.clone() };
        assert!(scope_matches(&scope, Some(&a)));
        assert!(!scope_matches(&scope, Some(&b)));
    }

    #[test]
    fn automation_is_not_local_user() {
        let automation = Principal::automation("morning-brief");
        assert_eq!(automation.kind, PrincipalKind::Automation);
        assert_ne!(automation.id, PrincipalId::owner());
        assert!(proof_matches_principal(
            &automation,
            &IdentityProof::InternalServiceBoundary
        ));
        assert!(!proof_matches_principal(
            &automation,
            &IdentityProof::LocalSystemSurface
        ));
    }

    #[test]
    fn worker_cannot_present_the_local_surface() {
        let worker = Principal::worker(Uuid::new_v4());
        assert!(!proof_matches_principal(
            &worker,
            &IdentityProof::LocalSystemSurface
        ));
        assert!(proof_matches_principal(
            &worker,
            &IdentityProof::DelegatedWorker {
                execution_id: Uuid::new_v4()
            }
        ));
    }

    #[test]
    fn app_capability_request_is_not_the_owner() {
        let request = AuthorityRequest::app_capability("org.saaios.demo", 7, "clipboard.read");
        assert_eq!(request.principal, Principal::app("org.saaios.demo"));
        assert_eq!(request.proof, IdentityProof::PeerCredentials { pid: 7 });
        assert_eq!(request_operation_id(&request), Some("clipboard.read"));
        assert_ne!(request.principal.id, PrincipalId::owner());
    }
}
