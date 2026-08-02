use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedToolCall {
    pub call_id: Uuid,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub assistant_text: Option<String>,
    pub tool_calls: Vec<ProposedToolCall>,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ModelResponse>;
}

/// Deterministic provider for e2e / `just run-mock`.
pub struct MockModelProvider;

#[async_trait]
impl ModelProvider for MockModelProvider {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        _tools: &[ToolDefinition],
    ) -> Result<ModelResponse> {
        let last = messages
            .last()
            .map(|m| m.content.to_lowercase())
            .unwrap_or_default();

        if last.contains("tool_result:system.metrics") {
            return Ok(ModelResponse {
                assistant_text: None,
                tool_calls: vec![ProposedToolCall {
                    call_id: Uuid::new_v4(),
                    name: "process.list".into(),
                    arguments: json!({}),
                }],
            });
        }

        if last.contains("tool_result:process.list") {
            return Ok(ModelResponse {
                assistant_text: Some(
                    "Процесс runaway-worker (pid 4312) использует 88% CPU и активно читает диск. Остальные метрики указывают на перегрузку CPU."
                        .into(),
                ),
                tool_calls: vec![ProposedToolCall {
                    call_id: Uuid::new_v4(),
                    name: "process.kill_request".into(),
                    arguments: json!({"pid": 4312}),
                }],
            });
        }

        if last.contains("тормоз") || last.contains("slow") || last.contains("cpu") {
            return Ok(ModelResponse {
                assistant_text: None,
                tool_calls: vec![ProposedToolCall {
                    call_id: Uuid::new_v4(),
                    name: "system.metrics".into(),
                    arguments: json!({}),
                }],
            });
        }

        Ok(ModelResponse {
            assistant_text: Some("Уточните, что проверить в системе.".into()),
            tool_calls: vec![],
        })
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    config: OpenAiCompatConfig,
}

impl OpenAiCompatProvider {
    pub fn new(config: OpenAiCompatConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait]
impl ModelProvider for OpenAiCompatProvider {
    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ModelResponse> {
        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );
        let tool_defs: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect();

        let body = json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tool_defs,
            "tool_choice": "auto"
        });

        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let value: Value = resp.json().await?;
        let message = value
            .pointer("/choices/0/message")
            .cloned()
            .ok_or_else(|| anyhow!("missing message in provider response"))?;

        let assistant_text = message
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let mut tool_calls = Vec::new();
        if let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) {
            for item in arr {
                let name = item
                    .pointer("/function/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let args_raw = item
                    .pointer("/function/arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let arguments: Value = serde_json::from_str(args_raw).unwrap_or(json!({}));
                tool_calls.push(ProposedToolCall {
                    call_id: Uuid::new_v4(),
                    name,
                    arguments,
                });
            }
        }

        Ok(ModelResponse {
            assistant_text,
            tool_calls,
        })
    }
}
