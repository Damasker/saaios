use protocol::{Envelope, MessageKind};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventClass {
    Critical,
    Lossy,
    Session,
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Envelope>,
    // Reserved for future classed queues / drop policies.
    _classes: Arc<RwLock<HashMap<String, EventClass>>>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            _classes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Envelope> {
        self.tx.subscribe()
    }

    pub fn publish(
        &self,
        kind: MessageKind,
        correlation_id: Uuid,
        causation_id: Option<Uuid>,
        payload: Value,
    ) -> Envelope {
        let env = Envelope::new(kind, correlation_id, causation_id, payload);
        // Lossy by default for 0.1 bus: ignore slow consumers.
        let _ = self.tx.send(env.clone());
        env
    }

    pub fn publish_envelope(&self, env: Envelope) {
        let _ = self.tx.send(env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn publish_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let corr = Uuid::new_v4();
        bus.publish(MessageKind::Event, corr, None, json!({"ok": true}));
        let got = rx.recv().await.unwrap();
        assert_eq!(got.correlation_id, corr);
    }
}
