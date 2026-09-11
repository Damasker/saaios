//! `saai-taskd`: S09 Change 2's minimal vertical slice. Watches one
//! space's `saai-entityd` feed for `saaios.intent` entities and turns
//! each into a `saaios.task` -> one non-dangerous `saaios.action` ->
//! `saaios.result`, per ADR-030's decision to model this natively on
//! `saai-entity-store` rather than on Platform Track's
//! `policy-engine`/`tool-registry`.
//!
//! Not yet wired into `native-init.c`'s boot sequence (unlike
//! `saai-appd`/`saai-entityd`) -- this Change proves the workflow
//! itself first, run manually, the same "prove it physically before
//! touching boot-critical init" order S06/S07 already followed for
//! their own daemons.

pub mod client;
pub mod model;

use client::{ClientError, EntitydConn};
use model::{
    action_properties, has_task_for_intent, result_properties, result_summary, run_echo_action,
    task_properties, WorkflowStatus, ACTION_TYPE, ECHO_ACTION_KIND, INTENT_TYPE, RESULT_TYPE,
    TASK_TYPE,
};
use saai_entity_protocol::{Entity, EntitydEvent};
use saai_entity_store::EventPayload;
use serde_json::{json, Value};
use std::path::Path;

pub struct Daemon {
    conn: EntitydConn,
    space_id: String,
    known_tasks: Vec<Entity>,
}

impl Daemon {
    pub async fn connect(socket: &Path, space_id: String) -> Result<Self, ClientError> {
        let mut conn = EntitydConn::connect(socket).await?;
        conn.subscribe().await?;
        let known_tasks = conn
            .list_entities(&space_id)
            .await?
            .into_iter()
            .filter(|entity| entity.entity_type == TASK_TYPE)
            .collect();
        Ok(Self {
            conn,
            space_id,
            known_tasks,
        })
    }

    /// Catches up on any `saaios.intent` created while this daemon
    /// wasn't running (or before it ever ran once) -- the same
    /// `intent_id`-on-`saaios.task` idempotency guard `run()` uses, so
    /// an intent already turned into a task on a previous run is never
    /// reprocessed.
    pub async fn reconcile_existing_intents(&mut self) -> Result<usize, ClientError> {
        let entities = self.conn.list_entities(&self.space_id).await?;
        let mut processed = 0;
        for intent in entities
            .into_iter()
            .filter(|entity| entity.entity_type == INTENT_TYPE)
        {
            if !has_task_for_intent(&self.known_tasks, intent.id) {
                self.process_intent(&intent).await?;
                processed += 1;
            }
        }
        Ok(processed)
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        loop {
            let EntitydEvent::EntityChanged { record } = self.conn.next_event().await? else {
                continue;
            };
            if record.space_id != self.space_id {
                continue;
            }
            let EventPayload::EntityCreated { entity } = record.payload else {
                continue;
            };
            if entity.entity_type != INTENT_TYPE {
                continue;
            }
            if has_task_for_intent(&self.known_tasks, entity.id) {
                continue;
            }
            self.process_intent(&entity).await?;
        }
    }

    async fn process_intent(&mut self, intent: &Entity) -> Result<(), ClientError> {
        let text = intent
            .properties
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or(&intent.title)
            .to_string();
        eprintln!("saai-taskd: intent {} -> \"{text}\"", intent.id);

        let task = self
            .conn
            .create_entity(
                &self.space_id,
                TASK_TYPE,
                &format!("Задача: {text}"),
                task_properties(intent.id, WorkflowStatus::Running),
            )
            .await?;
        self.known_tasks.push(task.clone());

        let input = json!({ "text": text });
        let action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &format!("Действие: {ECHO_ACTION_KIND}"),
                action_properties(
                    task.id,
                    ECHO_ACTION_KIND,
                    WorkflowStatus::Running,
                    &input,
                    None,
                ),
            )
            .await?;

        let output = run_echo_action(&text);
        let action = self
            .conn
            .update_entity(
                &action,
                action_properties(
                    task.id,
                    ECHO_ACTION_KIND,
                    WorkflowStatus::Done,
                    &input,
                    Some(&output),
                ),
            )
            .await?;

        let summary = result_summary(&text, &output);
        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &summary,
                result_properties(task.id, action.id, &summary),
            )
            .await?;

        let mut done_properties = task_properties(intent.id, WorkflowStatus::Done);
        done_properties.insert("result_id".into(), json!(result.id.to_string()));
        let updated_task = self.conn.update_entity(&task, done_properties).await?;
        if let Some(slot) = self
            .known_tasks
            .iter_mut()
            .find(|task| task.id == updated_task.id)
        {
            *slot = updated_task;
        }

        eprintln!("saai-taskd: intent {} done -> {summary}", intent.id);
        Ok(())
    }
}
