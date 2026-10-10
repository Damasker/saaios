use thiserror::Error;

/// Refusal produced before a probe is executed or a claim is stored.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContractError {
    #[error("campaign requires an intent document")]
    MissingIntent,
    #[error("intent document is not valid json")]
    InvalidIntent,
    #[error("intent is missing semantic_action_id")]
    MissingAction,
    #[error("action {action} does not open a hardware investigation")]
    OtherAction { action: String },
    #[error("intent source is missing or unknown")]
    InvalidSource,
    #[error("worker intent requires precondition_id")]
    WorkerRequiresPrecondition,
    #[error("probe request is not valid json")]
    InvalidProbe,
    #[error("probe is not in the read-only whitelist")]
    UnknownProbe,
    #[error("probe arguments do not match the probe schema")]
    InvalidArguments,
    #[error("probe argument contains shell metacharacters")]
    ShellMetacharacters,
    #[error("probe risk must be read_only")]
    RiskNotReadOnly,
    #[error("sysfs path escapes the allowlist")]
    PathEscape,
    #[error("pass limit reached; executor must not run")]
    PassLimit,
    #[error("claim has no evidence and cannot enter knowledge")]
    NoEvidence,
}
