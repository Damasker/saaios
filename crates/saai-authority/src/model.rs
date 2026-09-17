//! Identity, request, and reason codes. No evaluation.

use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrincipalId(pub String);

impl PrincipalId {
    pub fn owner() -> Self {
        Self("user:owner".into())
    }

    pub fn app(app_id: &str) -> Self {
        Self(format!("app:{app_id}"))
    }

    pub fn worker(execution_id: Uuid) -> Self {
        Self(format!("worker:{execution_id}"))
    }

    pub fn automation(id: &str) -> Self {
        Self(format!("automation:{id}"))
    }

    pub fn service(name: &str) -> Self {
        Self(format!("service:{name}"))
    }

    pub fn ssh_fingerprint(sha256: &str) -> Self {
        Self(format!("remote:ssh:{sha256}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    LocalUser,
    SystemService,
    Application,
    Worker,
    Automation,
    RemoteClient,
    /// Reserved for PCE. Not implemented.
    FutureNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub id: PrincipalId,
    pub kind: PrincipalKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IdentityProof {
    LocalSystemSurface,
    PeerCredentials { pid: u32 },
    InternalServiceBoundary,
    CryptographicKey { fingerprint: String },
    DelegatedWorker { execution_id: Uuid },
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthorityOperation {
    SemanticAction { action_id: String },
    AppCapabilityUse { capability: String },
    RemoteAdministration { service: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorityContext {
    pub space_ids: Vec<String>,
    pub focused_object: Option<ObjectRef>,
    pub workflow_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityCorrelation {
    pub intent_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub action_id: Option<Uuid>,
    pub call_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthorityRequest {
    pub principal: Principal,
    pub proof: IdentityProof,
    pub operation: AuthorityOperation,
    pub target: Option<ObjectRef>,
    pub context: AuthorityContext,
    pub arguments: Value,
    pub correlation: AuthorityCorrelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantValidity {
    OneShot,
    Session,
    Until(DateTime<Utc>),
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageConstraint {
    Once,
    UnlimitedWithinValidity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyReasonCode {
    HardDenied,
    UnknownOperation,
    IdentityUnverified,
    MissingGrant,
    ScopeMismatch,
    RequiresConfirmation,
    Granted,
    SessionGranted,
    StaticPolicyAllow,
}

impl AuthorityRequest {
    pub fn local_user_action(action_id: &str, target: Option<ObjectRef>, arguments: Value) -> Self {
        Self {
            principal: Principal {
                id: PrincipalId::owner(),
                kind: PrincipalKind::LocalUser,
            },
            proof: IdentityProof::LocalSystemSurface,
            operation: AuthorityOperation::SemanticAction {
                action_id: action_id.into(),
            },
            target,
            context: AuthorityContext {
                space_ids: Vec::new(),
                focused_object: None,
                workflow_id: None,
                session_id: None,
            },
            arguments,
            correlation: AuthorityCorrelation {
                intent_id: None,
                task_id: None,
                action_id: None,
                call_id: None,
            },
        }
    }
}

pub fn default_deny_unverified(proof: &IdentityProof) -> Option<PolicyReasonCode> {
    match proof {
        IdentityProof::Unverified => Some(PolicyReasonCode::IdentityUnverified),
        _ => None,
    }
}

pub fn default_deny_unknown_operation(known: bool) -> Option<PolicyReasonCode> {
    if known {
        None
    } else {
        Some(PolicyReasonCode::UnknownOperation)
    }
}
