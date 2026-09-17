use crate::spec::{ActionAvailability, ResolvedAction};
use policy_engine::{PolicyDecision, PolicyEngine};
use protocol::PolicyVerdict;
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

/// Policy preflight. Not an authorization token — execute re-checks.
pub fn preflight(
    policy: &PolicyEngine,
    tools: &ToolRegistry,
    action: &ResolvedAction,
) -> PolicyDecision {
    match tools.get(&action.tool_name) {
        Some(tool) => policy.decide_named(&action.tool_name, Some(tool.spec()), &action.arguments),
        None => policy.decide_named(&action.tool_name, None, &action.arguments),
    }
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
    let decision = policy.decide_named(&action.tool_name, Some(tool.spec()), &action.arguments);
    match decision.verdict {
        PolicyVerdict::Deny => Err(ExecuteError::Denied(decision.reason)),
        PolicyVerdict::AskUser => Err(ExecuteError::NeedsConfirmation(decision.reason)),
        PolicyVerdict::Allow => Ok(tool.execute(action.arguments.clone(), ctx).await?),
    }
}
