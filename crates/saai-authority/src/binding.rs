//! Canonical request binding. AUTH-04 may hash this; v1 compares equality.

use crate::model::{AuthorityOperation, AuthorityRequest};
use serde_json::{Map, Value};

pub fn canonical_binding(request: &AuthorityRequest) -> String {
    let mut obj = Map::new();
    obj.insert(
        "principal".into(),
        Value::String(request.principal.id.0.clone()),
    );
    obj.insert("operation".into(), operation_value(&request.operation));
    obj.insert(
        "target".into(),
        request
            .target
            .as_ref()
            .map(|t| serde_json::to_value(t).unwrap_or(Value::Null))
            .unwrap_or(Value::Null),
    );
    obj.insert("arguments".into(), canonicalize(&request.arguments));
    Value::Object(obj).to_string()
}

fn operation_value(op: &AuthorityOperation) -> Value {
    serde_json::to_value(op).unwrap_or(Value::Null)
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuthorityRequest;
    use saai_entity_store::ObjectRef;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn one_shot_rejects_changed_args() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let a = AuthorityRequest::local_user_action(
            "display.set_brightness",
            Some(target.clone()),
            json!({"percent": 40}),
        );
        let b = AuthorityRequest::local_user_action(
            "display.set_brightness",
            Some(target),
            json!({"percent": 100}),
        );
        assert_ne!(canonical_binding(&a), canonical_binding(&b));
    }

    #[test]
    fn one_shot_rejects_changed_target() {
        let a = AuthorityRequest::local_user_action(
            "service.restart",
            Some(ObjectRef::entity(Uuid::new_v4())),
            json!({}),
        );
        let b = AuthorityRequest::local_user_action(
            "service.restart",
            Some(ObjectRef::entity(Uuid::new_v4())),
            json!({}),
        );
        assert_ne!(canonical_binding(&a), canonical_binding(&b));
    }

    #[test]
    fn argument_key_order_does_not_change_binding() {
        let target = ObjectRef::entity(Uuid::new_v4());
        let a = AuthorityRequest::local_user_action(
            "test.mutate",
            Some(target.clone()),
            json!({"b": 1, "a": 2}),
        );
        let b = AuthorityRequest::local_user_action(
            "test.mutate",
            Some(target),
            json!({"a": 2, "b": 1}),
        );
        assert_eq!(canonical_binding(&a), canonical_binding(&b));
    }
}
