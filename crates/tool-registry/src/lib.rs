use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub risk: RiskLevel,
    pub timeout_ms: u64,
    pub input_schema: Value,
    pub output_schema: Value,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub correlation_id: Uuid,
    pub call_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub ok: bool,
    pub value: Value,
    pub error: Option<String>,
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("unknown tool: {0}")]
    Unknown(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    Execution(String),
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    fn spec(&self) -> &ToolSpec;
    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Arc<dyn ToolExecutor>) {
        let name = tool.spec().name.clone();
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolExecutor>> {
        self.tools.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec().clone()).collect()
    }

    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::Unknown(name.to_string()))?;
        if !args.is_object() && !args.is_null() {
            return Err(ToolError::InvalidArgs(
                "arguments must be object or null".into(),
            ));
        }
        tool.execute(args, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct EchoTool {
        spec: ToolSpec,
    }

    #[async_trait]
    impl ToolExecutor for EchoTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }

        async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput {
                ok: true,
                value: args,
                error: None,
            })
        }
    }

    #[tokio::test]
    async fn register_and_execute() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool {
            spec: ToolSpec {
                name: "echo".into(),
                description: "echo".into(),
                risk: RiskLevel::Low,
                timeout_ms: 1000,
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
                requires_confirmation: false,
            },
        }));
        let out = reg
            .execute(
                "echo",
                json!({"a":1}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap();
        assert!(out.ok);
        assert_eq!(out.value["a"], 1);
    }
}
