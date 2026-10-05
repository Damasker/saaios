use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResolutionOutcome {
    Answer(AnswerResolution),
    Action(ActionResolution),
    Plan(PlanResolution),
    Clarification(ClarificationResolution),
    Unsupported(UnsupportedResolution),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnswerResolution {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionResolution {
    pub target: ObjectRef,
    pub action_id: String,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_revision: Option<u64>,
}

impl ActionResolution {
    pub fn is_stale(&self, current_revision: u64) -> bool {
        self.target_revision
            .is_some_and(|captured| captured != current_revision)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanResolution {
    pub goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClarificationOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_ref: Option<ObjectRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClarificationResolution {
    pub question: String,
    pub options: Vec<ClarificationOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedResolution {
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMethod {
    ExplicitAction,
    FocusedObject,
    Relationship,
    DeterministicRule,
    Model,
    UserClarification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionTrace {
    pub target_source: ResolutionMethod,
    pub action_source: ResolutionMethod,
    pub clarification_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveAttempt {
    Resolved {
        outcome: ResolutionOutcome,
        trace: ResolutionTrace,
    },
    NeedsModel,
}
