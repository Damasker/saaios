//! ADR-121 WORK-01: a `PlanProposal` is an untrusted structured DAG.
//! Validate it deterministically before any Task is persisted.
//!
//! This module does not write a Ready status and does not talk to the
//! model. Linear S09/S10 Intent → one Task is admitted by WORK-02's
//! derived ready set (ADR-237).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

/// Starting evidence values (spec §61), not product constants.
pub const PROPOSAL_SCHEMA_V1: u32 = 1;
pub const MAX_TASKS_PER_INTENT: usize = 16;
pub const MAX_DEPENDENCY_DEPTH: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProposal {
    pub schema: u32,
    pub goal: String,
    pub tasks: Vec<ProposedTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedTask {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub action_id: Option<String>,
    #[serde(default)]
    pub target: Option<Value>,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPlan {
    pub goal: String,
    /// Unique proposal ids in topological order (parents before children).
    pub order: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanError {
    #[error("unsupported proposal schema {0}")]
    UnsupportedSchema(u32),
    #[error("plan has no tasks")]
    EmptyPlan,
    #[error("empty proposal id")]
    EmptyProposalId,
    #[error("duplicate proposal id {0}")]
    DuplicateProposalId(String),
    #[error("task {task} depends on missing {missing}")]
    MissingDependency { task: String, missing: String },
    #[error("plan graph contains a cycle")]
    Cycle,
    #[error("plan has {count} tasks; limit is {limit}")]
    TooManyTasks { count: usize, limit: usize },
    #[error("dependency depth {depth} exceeds limit {limit}")]
    DependencyTooDeep { depth: usize, limit: usize },
}

pub fn validate_plan(proposal: &PlanProposal) -> Result<ValidatedPlan, PlanError> {
    if proposal.schema != PROPOSAL_SCHEMA_V1 {
        return Err(PlanError::UnsupportedSchema(proposal.schema));
    }
    if proposal.tasks.is_empty() {
        return Err(PlanError::EmptyPlan);
    }
    if proposal.tasks.len() > MAX_TASKS_PER_INTENT {
        return Err(PlanError::TooManyTasks {
            count: proposal.tasks.len(),
            limit: MAX_TASKS_PER_INTENT,
        });
    }

    let mut by_id: HashMap<&str, &ProposedTask> = HashMap::new();
    for task in &proposal.tasks {
        if task.id.is_empty() {
            return Err(PlanError::EmptyProposalId);
        }
        if by_id.insert(task.id.as_str(), task).is_some() {
            return Err(PlanError::DuplicateProposalId(task.id.clone()));
        }
    }

    for task in &proposal.tasks {
        for dep in &task.depends_on {
            if !by_id.contains_key(dep.as_str()) {
                return Err(PlanError::MissingDependency {
                    task: task.id.clone(),
                    missing: dep.clone(),
                });
            }
            if dep == &task.id {
                return Err(PlanError::Cycle);
            }
        }
    }

    let order = topological_order(&proposal.tasks)?;
    let depth = longest_path_depth(&proposal.tasks);
    if depth > MAX_DEPENDENCY_DEPTH {
        return Err(PlanError::DependencyTooDeep {
            depth,
            limit: MAX_DEPENDENCY_DEPTH,
        });
    }

    Ok(ValidatedPlan {
        goal: proposal.goal.clone(),
        order,
    })
}

fn topological_order(tasks: &[ProposedTask]) -> Result<Vec<String>, PlanError> {
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for task in tasks {
        indegree.entry(task.id.as_str()).or_insert(0);
        children.entry(task.id.as_str()).or_default();
    }
    for task in tasks {
        for dep in &task.depends_on {
            *indegree.get_mut(task.id.as_str()).expect("id present") += 1;
            children
                .get_mut(dep.as_str())
                .expect("dep present")
                .push(task.id.as_str());
        }
    }

    let mut roots: Vec<&str> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    roots.sort_unstable();
    let mut queue: VecDeque<&str> = roots.into();

    let mut order = Vec::with_capacity(tasks.len());
    while let Some(id) = queue.pop_front() {
        order.push(id.to_string());
        let mut next: Vec<&str> = children.get(id).cloned().unwrap_or_default();
        next.sort();
        for child in next {
            let entry = indegree.get_mut(child).expect("child present");
            *entry -= 1;
            if *entry == 0 {
                queue.push_back(child);
            }
        }
    }

    if order.len() != tasks.len() {
        return Err(PlanError::Cycle);
    }
    Ok(order)
}

/// Longest path in edges (a → b is one hop). Isolated nodes have depth 0.
fn longest_path_depth(tasks: &[ProposedTask]) -> usize {
    let mut memo: HashMap<&str, usize> = HashMap::new();
    let by_id: HashMap<&str, &ProposedTask> =
        tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    fn depth<'a>(
        id: &'a str,
        by_id: &HashMap<&str, &'a ProposedTask>,
        memo: &mut HashMap<&'a str, usize>,
        visiting: &mut HashSet<&'a str>,
    ) -> usize {
        if let Some(known) = memo.get(id) {
            return *known;
        }
        if !visiting.insert(id) {
            return 0;
        }
        let task = by_id[id];
        let d = task
            .depends_on
            .iter()
            .map(|dep| 1 + depth(dep, by_id, memo, visiting))
            .max()
            .unwrap_or(0);
        visiting.remove(id);
        memo.insert(id, d);
        d
    }
    let mut visiting = HashSet::new();
    tasks
        .iter()
        .map(|t| depth(t.id.as_str(), &by_id, &mut memo, &mut visiting))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, deps: &[&str]) -> ProposedTask {
        ProposedTask {
            id: id.into(),
            title: id.into(),
            action_id: None,
            target: None,
            parameters: Value::Null,
            depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        }
    }

    fn proposal(tasks: Vec<ProposedTask>) -> PlanProposal {
        PlanProposal {
            schema: PROPOSAL_SCHEMA_V1,
            goal: "test".into(),
            tasks,
        }
    }

    #[test]
    fn dag_accepts_acyclic_graph() {
        let plan = validate_plan(&proposal(vec![
            task("t1", &[]),
            task("t2", &["t1"]),
            task("t3", &["t1"]),
            task("t4", &["t2", "t3"]),
        ]))
        .expect("acyclic");
        let pos = |id: &str| {
            plan.order
                .iter()
                .position(|x| x == id)
                .expect("id in order")
        };
        assert!(pos("t1") < pos("t2"));
        assert!(pos("t1") < pos("t3"));
        assert!(pos("t2") < pos("t4"));
        assert!(pos("t3") < pos("t4"));
    }

    #[test]
    fn dag_rejects_cycle() {
        assert_eq!(
            validate_plan(&proposal(vec![task("t1", &["t2"]), task("t2", &["t1"])])),
            Err(PlanError::Cycle)
        );
    }

    #[test]
    fn dag_rejects_self_cycle() {
        assert_eq!(
            validate_plan(&proposal(vec![task("t1", &["t1"])])),
            Err(PlanError::Cycle)
        );
    }

    #[test]
    fn dag_rejects_missing_dependency() {
        assert_eq!(
            validate_plan(&proposal(vec![task("t1", &[]), task("t2", &["ghost"])])),
            Err(PlanError::MissingDependency {
                task: "t2".into(),
                missing: "ghost".into(),
            })
        );
    }

    #[test]
    fn dag_rejects_duplicate_proposal_ids() {
        assert_eq!(
            validate_plan(&proposal(vec![task("t1", &[]), task("t1", &[])])),
            Err(PlanError::DuplicateProposalId("t1".into()))
        );
    }

    #[test]
    fn task_count_limit_enforced() {
        let tasks: Vec<_> = (0..MAX_TASKS_PER_INTENT + 1)
            .map(|i| task(&format!("t{i}"), &[]))
            .collect();
        assert_eq!(
            validate_plan(&proposal(tasks)),
            Err(PlanError::TooManyTasks {
                count: MAX_TASKS_PER_INTENT + 1,
                limit: MAX_TASKS_PER_INTENT,
            })
        );
    }

    #[test]
    fn fan_out_bounded_by_task_count() {
        let mut tasks = vec![task("root", &[])];
        for i in 0..MAX_TASKS_PER_INTENT {
            tasks.push(task(&format!("c{i}"), &["root"]));
        }
        assert!(matches!(
            validate_plan(&proposal(tasks)),
            Err(PlanError::TooManyTasks { .. })
        ));
    }

    #[test]
    fn proposal_json_schema_v1_parses() {
        let raw = serde_json::json!({
            "schema": 1,
            "goal": "Deploy test environment",
            "tasks": [{
                "id": "provision",
                "title": "Provision server",
                "action_id": "infrastructure.provision",
                "target": null,
                "parameters": {},
                "depends_on": []
            }]
        });
        let proposal: PlanProposal = serde_json::from_value(raw).expect("parse");
        validate_plan(&proposal).expect("valid");
    }

    #[test]
    fn do_not_parse_prose_as_graph() {
        let prose = PlanProposal {
            schema: 1,
            goal: "First do X, then maybe Y".into(),
            tasks: vec![],
        };
        assert_eq!(validate_plan(&prose), Err(PlanError::EmptyPlan));
    }
}
