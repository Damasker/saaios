use model_provider::ChatMessage;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

/// Process-local multi-turn chat history (orthogonal to policy session grants).
pub struct ConversationStore {
    inner: Mutex<HashMap<Uuid, Vec<ChatMessage>>>,
    max_messages: usize,
}

impl ConversationStore {
    pub fn new(max_messages: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max_messages: max_messages.max(4),
        }
    }

    pub fn session_count(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn get(&self, session_id: Uuid) -> Vec<ChatMessage> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.get(&session_id).cloned())
            .unwrap_or_default()
    }

    pub fn save(&self, session_id: Uuid, messages: Vec<ChatMessage>) {
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        let trimmed = trim_messages(messages, self.max_messages);
        if trimmed.is_empty() {
            guard.remove(&session_id);
        } else {
            guard.insert(session_id, trimmed);
        }
    }

    pub fn append(&self, session_id: Uuid, message: ChatMessage) {
        let Ok(mut guard) = self.inner.lock() else {
            return;
        };
        let entry = guard.entry(session_id).or_default();
        entry.push(message);
        *entry = trim_messages(std::mem::take(entry), self.max_messages);
    }

    pub fn reset(&self, session_id: Uuid) -> bool {
        self.inner
            .lock()
            .ok()
            .map(|mut g| g.remove(&session_id).is_some())
            .unwrap_or(false)
    }

    pub fn clear_all(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.clear();
        }
    }
}

fn trim_messages(mut messages: Vec<ChatMessage>, max: usize) -> Vec<ChatMessage> {
    if messages.len() <= max {
        return messages;
    }
    let drop = messages.len() - max;
    messages.drain(0..drop);
    // Avoid starting mid tool_result without prior assistant context when possible.
    while messages
        .first()
        .is_some_and(|m| m.role == "tool" && m.content.starts_with("tool_result:"))
    {
        messages.remove(0);
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_oldest_and_skips_orphan_tool_results() {
        let store = ConversationStore::new(4);
        let sid = Uuid::new_v4();
        let msgs: Vec<_> = (0..6)
            .map(|i| {
                ChatMessage::text(
                    if i % 2 == 0 { "user" } else { "assistant" },
                    format!("m{i}"),
                )
            })
            .collect();
        store.save(sid, msgs);
        assert_eq!(store.get(sid).len(), 4);
        assert_eq!(store.get(sid)[0].content, "m2");
    }
}
