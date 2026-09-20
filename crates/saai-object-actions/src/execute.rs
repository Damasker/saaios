use crate::spec::{ActionAvailability, ResolvedAction};
use policy_engine::{PolicyDecision, PolicyEngine};
use protocol::PolicyVerdict;
use saai_authority::{AuthorityRequest, IdentityProof, Principal};
use saai_entity_store::Entity;
use thiserror::Error;
use tool_registry::{ToolContext, ToolError, ToolOutput, ToolRegistry};

#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("action is unavailable: {0}")]
    Unavailable(String),
    #[error("policy denied: {0}")]
    Denied(String),
    #[error("policy requires confirmation: {0}")]
    NeedsConfirmation(String),
    #[error("resolved action is stale for the current object")]
    StaleObject,
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error(transparent)]
    Tool(#[from] ToolError),
}

/// AUTH-05: OAM names the semantic action, bound tool, target, and Principal.
pub fn authority_request(
    action: &ResolvedAction,
    principal: Principal,
    proof: IdentityProof,
) -> AuthorityRequest {
    let mut request = AuthorityRequest::local_user_action(
        &action.action_id,
        Some(action.target.clone()),
        action.arguments.clone(),
    );
    request.principal = principal;
    request.proof = proof;
    request.context.focused_object = Some(action.target.clone());
    request
}

fn local_user_request(action: &ResolvedAction) -> AuthorityRequest {
    authority_request(
        action,
        Principal::local_user(),
        IdentityProof::LocalSystemSurface,
    )
}

/// Policy preflight. Not an authorization token — execute re-checks.
pub fn preflight(
    policy: &PolicyEngine,
    tools: &ToolRegistry,
    action: &ResolvedAction,
) -> PolicyDecision {
    preflight_for(policy, tools, action, &local_user_request(action))
}

pub fn preflight_for(
    policy: &PolicyEngine,
    tools: &ToolRegistry,
    action: &ResolvedAction,
    request: &AuthorityRequest,
) -> PolicyDecision {
    let tool = tools.get(&action.tool_name);
    policy.decide_request(request, tool.as_ref().map(|t| t.spec()))
}

/// Re-evaluates policy on the live engine, then runs the tool only on Allow.
/// Production workflow still goes through Intent/Task/Action; this is the
/// shared gate so OAM cannot bypass Deny/AskUser in tests or later bridges.
pub async fn execute_if_allowed(
    policy: &PolicyEngine,
    tools: &ToolRegistry,
    action: &ResolvedAction,
    current: &Entity,
    ctx: &ToolContext,
) -> Result<ToolOutput, ExecuteError> {
    execute_if_allowed_for(
        policy,
        tools,
        action,
        current,
        ctx,
        &local_user_request(action),
    )
    .await
}

pub async fn execute_if_allowed_for(
    policy: &PolicyEngine,
    tools: &ToolRegistry,
    action: &ResolvedAction,
    current: &Entity,
    ctx: &ToolContext,
    request: &AuthorityRequest,
) -> Result<ToolOutput, ExecuteError> {
    if action.is_stale(current) {
        return Err(ExecuteError::StaleObject);
    }
    match &action.availability {
        ActionAvailability::Unavailable { reason } => {
            return Err(ExecuteError::Unavailable(reason.clone()));
        }
        ActionAvailability::Available => {}
    }
    let tool = tools
        .get(&action.tool_name)
        .ok_or_else(|| ExecuteError::UnknownTool(action.tool_name.clone()))?;
    let decision = policy.decide_request(request, Some(tool.spec()));
    match decision.verdict {
        PolicyVerdict::Deny => Err(ExecuteError::Denied(decision.reason)),
        PolicyVerdict::AskUser => Err(ExecuteError::NeedsConfirmation(decision.reason)),
        PolicyVerdict::Allow => Ok(tool.execute(action.arguments.clone(), ctx).await?),
    }
}
