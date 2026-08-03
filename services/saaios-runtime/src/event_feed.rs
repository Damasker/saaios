//! In-process ring buffer of recent automation / system events for TUI/UDS.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub ts: String,
    pub event: String,
    pub correlation_id: Option<Uuid>,
    pub summary: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug)]
pub struct EventFeed {
    items: Mutex<VecDeque<FeedItem>>,
    capacity: usize,
}

impl EventFeed {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            capacity: capacity.max(1),
        }
    }

    pub fn push(&self, item: FeedItem) {
        let mut guard = self.items.lock().expect("event feed lock");
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(item);
    }

    pub fn tail(&self, limit: usize) -> Vec<FeedItem> {
        let guard = self.items.lock().expect("event feed lock");
        let n = limit.min(guard.len());
        guard
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ring_drops_oldest() {
        let feed = EventFeed::new(2);
        for i in 0..3 {
            feed.push(FeedItem {
                ts: format!("{i}"),
                event: "x".into(),
                correlation_id: None,
                summary: format!("{i}"),
                payload: json!({}),
            });
        }
        let tail = feed.tail(10);
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].summary, "1");
        assert_eq!(tail[1].summary, "2");
    }
}
