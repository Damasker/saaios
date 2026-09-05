use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ChatToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tools(content: Option<String>, calls: &[ProposedToolCall]) -> Self {
        Self {
            role: "assistant".into(),
            content: content.unwrap_or_default(),
            tool_calls: calls.iter().map(ChatToolCall::from).collect(),
            tool_call_id: None,
        }
    }

    pub fn tool_result(call_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_calls: Vec::new(),
            tool_call_id: Some(call_id.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ChatToolFunction,
}

impl From<&ProposedToolCall> for ChatToolCall {
    fn from(call: &ProposedToolCall) -> Self {
        Self {
            id: call.call_id.to_string(),
            kind: "function".into(),
            function: ChatToolFunction {
                name: call.name.clone(),
                arguments: serde_json::to_string(&call.arguments).unwrap_or_else(|_| "{}".into()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolFunction {
    pub name: String,
    pub arguments: String,
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

/// Incremental text from a streaming completion (token / chunk deltas).
pub type TextDeltaTx = tokio::sync::mpsc::UnboundedSender<String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Mock,
    Remote,
    Local,
    Auto,
}

impl ProviderKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "mock" => Some(Self::Mock),
            "remote" => Some(Self::Remote),
            "local" | "ollama" => Some(Self::Local),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str {
        "provider"
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ModelResponse>;

    /// Like [`complete`], optionally emitting text deltas via `delta_tx`.
    /// Default implementation calls `complete` once, then forwards the full text.
    async fn complete_with_progress(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        delta_tx: Option<&TextDeltaTx>,
    ) -> Result<ModelResponse> {
        let resp = self.complete(messages, tools).await?;
        if let (Some(tx), Some(text)) = (delta_tx, resp.assistant_text.as_ref()) {
            let _ = tx.send(text.clone());
        }
        Ok(resp)
    }
}

/// Deterministic provider for e2e / `just run-mock`.
pub struct MockModelProvider;

#[async_trait]
impl ModelProvider for MockModelProvider {
    fn name(&self) -> &str {
        "mock"
    }

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

        // Multi-turn follow-ups that reuse prior diagnose context in the transcript.
        let joined = messages
            .iter()
            .map(|m| m.content.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");
        if joined.contains("runaway-worker")
            && (last.contains("pid")
                || last.contains("какой")
                || last.contains("who")
                || last.contains("which")
                || last.contains("имя")
                || last.contains("name"))
        {
            return Ok(ModelResponse {
                assistant_text: Some("Ранее найденный виновник — runaway-worker, pid 4312.".into()),
                tool_calls: vec![],
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

    async fn complete_with_progress(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        delta_tx: Option<&TextDeltaTx>,
    ) -> Result<ModelResponse> {
        let resp = self.complete(messages, tools).await?;
        if let (Some(tx), Some(text)) = (delta_tx, resp.assistant_text.as_ref()) {
            // Word-sized chunks so mock demos exercise the streaming path.
            for (i, word) in text.split_inclusive(' ').enumerate() {
                if i > 0 {
                    tokio::time::sleep(Duration::from_millis(8)).await;
                }
                let _ = tx.send(word.to_string());
            }
        }
        Ok(resp)
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
    pub label: String,
}

impl OpenAiCompatConfig {
    pub fn remote(api_base: String, api_key: String, model: String) -> Self {
        Self {
            api_base,
            api_key,
            model,
            label: "remote".into(),
        }
    }

    pub fn ollama(api_base: String, model: String) -> Self {
        Self {
            api_base,
            api_key: "ollama".into(),
            model,
            label: "local-ollama".into(),
        }
    }
}

pub struct OpenAiCompatProvider {
    client: reqwest::Client,
    config: OpenAiCompatConfig,
}

impl OpenAiCompatProvider {
    pub fn new(config: OpenAiCompatConfig) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            config,
        }
    }

    pub fn ollama_default(model: impl Into<String>) -> Self {
        Self::new(OpenAiCompatConfig::ollama(
            "http://127.0.0.1:11434/v1".into(),
            model.into(),
        ))
    }
}

#[async_trait]
impl ModelProvider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        &self.config.label
    }

    async fn health_check(&self) -> Result<()> {
        // Ollama native health endpoint; for generic OpenAI-compat try models list.
        if self.config.label.contains("ollama") || self.config.api_base.contains("11434") {
            let base = self
                .config
                .api_base
                .trim_end_matches('/')
                .trim_end_matches("/v1");
            let url = format!("{base}/api/tags");
            let resp = self.client.get(url).send().await?.error_for_status()?;
            let _body: Value = resp.json().await?;
            return Ok(());
        }
        let url = format!("{}/models", self.config.api_base.trim_end_matches('/'));
        let mut req = self.client.get(url);
        if !self.config.api_key.is_empty() {
            req = req.bearer_auth(&self.config.api_key);
        }
        req.send().await?.error_for_status()?;
        Ok(())
    }

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

        let mut req = self.client.post(url).json(&body);
        if !self.config.api_key.is_empty() {
            req = req.bearer_auth(&self.config.api_key);
        }
        let resp = req.send().await?.error_for_status()?;
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

    async fn complete_with_progress(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        delta_tx: Option<&TextDeltaTx>,
    ) -> Result<ModelResponse> {
        // No sink → non-streaming path.
        let Some(tx) = delta_tx else {
            return self.complete(messages, tools).await;
        };

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
            "tool_choice": "auto",
            "stream": true
        });

        let mut req = self.client.post(url).json(&body);
        if !self.config.api_key.is_empty() {
            req = req.bearer_auth(&self.config.api_key);
        }
        let resp = req.send().await?.error_for_status()?;

        use futures::StreamExt;
        let mut byte_stream = resp.bytes_stream();
        let mut line_buf = String::new();
        let mut assistant_text = String::new();
        // index -> (name, arguments_json_fragments)
        let mut tool_acc: Vec<(String, String)> = Vec::new();

        while let Some(chunk) = byte_stream.next().await {
            let chunk = chunk.map_err(|e| anyhow!("stream read: {e}"))?;
            line_buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = line_buf.find('\n') {
                let mut line = line_buf[..pos].to_string();
                line_buf = line_buf[pos + 1..].to_string();
                if line.ends_with('\r') {
                    line.pop();
                }
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                let value: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let Some(delta) = value.pointer("/choices/0/delta") else {
                    continue;
                };
                if let Some(piece) = delta.get("content").and_then(|c| c.as_str()) {
                    if !piece.is_empty() {
                        assistant_text.push_str(piece);
                        let _ = tx.send(piece.to_string());
                    }
                }
                if let Some(arr) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for item in arr {
                        let idx = item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                        while tool_acc.len() <= idx {
                            tool_acc.push((String::new(), String::new()));
                        }
                        if let Some(name) = item.pointer("/function/name").and_then(|v| v.as_str())
                        {
                            if !name.is_empty() {
                                tool_acc[idx].0 = name.to_string();
                            }
                        }
                        if let Some(args) =
                            item.pointer("/function/arguments").and_then(|v| v.as_str())
                        {
                            tool_acc[idx].1.push_str(args);
                        }
                    }
                }
            }
        }

        let tool_calls = tool_acc
            .into_iter()
            .filter(|(name, _)| !name.is_empty())
            .map(|(name, args_raw)| {
                let arguments: Value = serde_json::from_str(&args_raw).unwrap_or(json!({}));
                ProposedToolCall {
                    call_id: Uuid::new_v4(),
                    name,
                    arguments,
                }
            })
            .collect();

        Ok(ModelResponse {
            assistant_text: if assistant_text.is_empty() {
                None
            } else {
                Some(assistant_text)
            },
            tool_calls,
        })
    }
}

/// Tries providers in order; first healthy/successful wins per call.
pub struct FallbackProvider {
    providers: Vec<Arc<dyn ModelProvider>>,
}

impl FallbackProvider {
    pub fn new(providers: Vec<Arc<dyn ModelProvider>>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl ModelProvider for FallbackProvider {
    fn name(&self) -> &str {
        "fallback"
    }

    async fn health_check(&self) -> Result<()> {
        for p in &self.providers {
            if p.health_check().await.is_ok() {
                return Ok(());
            }
        }
        Err(anyhow!("no healthy providers in fallback chain"))
    }

    async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
    ) -> Result<ModelResponse> {
        let mut last_err = None;
        for p in &self.providers {
            match p.complete(messages, tools).await {
                Ok(resp) => {
                    info!(provider = p.name(), "model complete succeeded");
                    return Ok(resp);
                }
                Err(e) => {
                    warn!(provider = p.name(), error = %e, "model complete failed; trying next");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("fallback provider chain empty")))
    }

    async fn complete_with_progress(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        delta_tx: Option<&TextDeltaTx>,
    ) -> Result<ModelResponse> {
        let mut last_err = None;
        for p in &self.providers {
            match p.complete_with_progress(messages, tools, delta_tx).await {
                Ok(resp) => {
                    info!(provider = p.name(), "model stream complete succeeded");
                    return Ok(resp);
                }
                Err(e) => {
                    warn!(provider = p.name(), error = %e, "model stream failed; trying next");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow!("fallback provider chain empty")))
    }
}

/// Build provider from kind + env/config.
pub async fn build_provider(
    kind: ProviderKind,
    remote_base: Option<String>,
    remote_key: Option<String>,
    remote_model: Option<String>,
    local_base: Option<String>,
    local_model: Option<String>,
) -> Result<Arc<dyn ModelProvider>> {
    let remote_model = remote_model.unwrap_or_else(|| "gpt-4o-mini".into());
    let local_model = local_model.unwrap_or_else(|| "llama3.2".into());
    let local_base = local_base.unwrap_or_else(|| "http://127.0.0.1:11434/v1".into());

    match kind {
        ProviderKind::Mock => Ok(Arc::new(MockModelProvider)),
        ProviderKind::Remote => {
            let api_base = remote_base.ok_or_else(|| anyhow!("remote api base required"))?;
            let api_key = remote_key.ok_or_else(|| anyhow!("remote api key required"))?;
            Ok(Arc::new(OpenAiCompatProvider::new(
                OpenAiCompatConfig::remote(api_base, api_key, remote_model),
            )))
        }
        ProviderKind::Local => {
            let p = OpenAiCompatProvider::new(OpenAiCompatConfig::ollama(local_base, local_model));
            if let Err(e) = p.health_check().await {
                warn!(error = %e, "local provider health check failed; continuing anyway");
            }
            Ok(Arc::new(p))
        }
        ProviderKind::Auto => {
            let mut chain: Vec<Arc<dyn ModelProvider>> = Vec::new();
            let local = OpenAiCompatProvider::new(OpenAiCompatConfig::ollama(
                local_base.clone(),
                local_model,
            ));
            if local.health_check().await.is_ok() {
                info!("auto provider: local ollama healthy");
                chain.push(Arc::new(local));
            } else {
                warn!("auto provider: local ollama unavailable");
            }
            if let (Some(base), Some(key)) = (remote_base, remote_key) {
                chain.push(Arc::new(OpenAiCompatProvider::new(
                    OpenAiCompatConfig::remote(base, key, remote_model),
                )));
            }
            chain.push(Arc::new(MockModelProvider));
            Ok(Arc::new(FallbackProvider::new(chain)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_streams_text_deltas() {
        let p = MockModelProvider;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let resp = p
            .complete_with_progress(&[ChatMessage::text("user", "hello")], &[], Some(&tx))
            .await
            .unwrap();
        drop(tx);
        let mut pieces = Vec::new();
        while let Some(s) = rx.recv().await {
            pieces.push(s);
        }
        assert!(resp.assistant_text.as_ref().unwrap().contains("Уточните"));
        assert!(pieces.len() > 1, "expected chunked deltas, got {pieces:?}");
        assert_eq!(pieces.concat(), resp.assistant_text.unwrap());
    }

    #[tokio::test]
    async fn mock_provider_diagnose_path() {
        let p = MockModelProvider;
        let resp = p
            .complete(&[ChatMessage::text("user", "Почему тормозит?")], &[])
            .await
            .unwrap();
        assert_eq!(resp.tool_calls[0].name, "system.metrics");
    }

    #[tokio::test]
    async fn fallback_uses_second_when_first_fails() {
        struct Boom;
        #[async_trait]
        impl ModelProvider for Boom {
            fn name(&self) -> &str {
                "boom"
            }
            async fn complete(
                &self,
                _: &[ChatMessage],
                _: &[ToolDefinition],
            ) -> Result<ModelResponse> {
                Err(anyhow!("boom"))
            }
        }

        let fb = FallbackProvider::new(vec![Arc::new(Boom), Arc::new(MockModelProvider)]);
        let resp = fb
            .complete(&[ChatMessage::text("user", "hello")], &[])
            .await
            .unwrap();
        assert!(resp.assistant_text.is_some());
    }

    #[tokio::test]
    async fn build_mock_provider() {
        let p = build_provider(ProviderKind::Mock, None, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(p.name(), "mock");
    }

    #[test]
    fn serializes_openai_tool_exchange() {
        let call = ProposedToolCall {
            call_id: Uuid::nil(),
            name: "system.metrics".into(),
            arguments: json!({}),
        };
        let assistant = serde_json::to_value(ChatMessage::assistant_with_tools(
            None,
            std::slice::from_ref(&call),
        ))
        .unwrap();
        assert_eq!(assistant["role"], "assistant");
        assert_eq!(assistant["tool_calls"][0]["id"], Uuid::nil().to_string());
        assert_eq!(
            assistant["tool_calls"][0]["function"]["name"],
            "system.metrics"
        );

        let result = serde_json::to_value(ChatMessage::tool_result(
            call.call_id,
            "tool_result:system.metrics {}",
        ))
        .unwrap();
        assert_eq!(result["role"], "tool");
        assert_eq!(result["tool_call_id"], Uuid::nil().to_string());
        assert!(result.get("tool_calls").is_none());
    }

    #[test]
    fn provider_kind_parse() {
        assert_eq!(ProviderKind::parse("ollama"), Some(ProviderKind::Local));
        assert_eq!(ProviderKind::parse("AUTO"), Some(ProviderKind::Auto));
        assert_eq!(ProviderKind::parse("nope"), None);
    }
}
