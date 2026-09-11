//! `saai-taskd`: S09's minimal vertical slice. Watches one space's
//! `saai-entityd` feed for `saaios.intent` entities and turns each into
//! a `saaios.task` -> `saaios.action` -> `saaios.result`, per ADR-030's
//! decision to model this natively on `saai-entity-store` rather than
//! on Platform Track's `policy-engine`/`tool-registry`.
//!
//! Change 2 built the non-dangerous path (one deterministic `echo`
//! Action, executed immediately). Change 3 (ADR-031's follow-up) adds
//! the dangerous path: an intent that selects `delete_entity` pauses at
//! `WaitingConfirmation` instead of running -- this daemon never
//! transitions a Task out of that state itself. Only an external write
//! (a live touch on `saai-shell`'s confirmation screen) can move it to
//! `Running`, and this daemon then finishes the chain reactively. A
//! restart re-enters that same reactive wait, per S09's Threat/privacy
//! impact requirement that a resumed Task never auto-executes on a
//! cached decision.
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
    action_properties, dangerous_action_of, find_action_for_task, has_task_for_intent,
    result_properties, result_summary, run_echo_action, status_of, task_properties, WorkflowStatus,
    ACTION_TYPE, DELETE_ENTITY_ACTION_KIND, ECHO_ACTION_KIND, INTENT_TYPE, RESULT_TYPE, TASK_TYPE,
};
use saai_entity_protocol::{Entity, EntitydEvent};
use saai_entity_store::EventPayload;
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

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

    /// Catches up on any Task that was confirmed (moved to `Running`)
    /// or declined (moved to `Cancelled`) while this daemon wasn't
    /// running to see the event -- the crash-between-decision-and-
    /// effect edge case. Deliberately does *nothing* for a Task still
    /// sitting in `WaitingConfirmation`: that silence is the point
    /// (S09's Threat/privacy impact requirement) -- this method only
    /// ever acts on a decision a human already made, never on one
    /// merely pending.
    pub async fn reconcile_confirmed_tasks(&mut self) -> Result<usize, ClientError> {
        let decided: Vec<Entity> = self
            .known_tasks
            .iter()
            .filter(|task| {
                matches!(
                    status_of(task),
                    Some(WorkflowStatus::Running) | Some(WorkflowStatus::Cancelled)
                )
            })
            .cloned()
            .collect();
        let mut resumed = 0;
        for task in decided {
            match status_of(&task) {
                Some(WorkflowStatus::Running) => {
                    if self.try_resume_confirmed_task(&task).await? {
                        resumed += 1;
                    }
                }
                Some(WorkflowStatus::Cancelled) => {
                    self.try_cancel_pending_action(&task).await?;
                }
                _ => {}
            }
        }
        Ok(resumed)
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        loop {
            let EntitydEvent::EntityChanged { record } = self.conn.next_event().await? else {
                continue;
            };
            if record.space_id != self.space_id {
                continue;
            }
            let entity = match record.payload {
                EventPayload::EntityCreated { entity } | EventPayload::EntityUpdated { entity } => {
                    entity
                }
                EventPayload::SpaceCreated { .. } | EventPayload::EntityDeleted { .. } => continue,
            };
            if entity.entity_type == INTENT_TYPE {
                if !has_task_for_intent(&self.known_tasks, entity.id) {
                    self.process_intent(&entity).await?;
                }
            } else if entity.entity_type == TASK_TYPE {
                self.remember_task(entity.clone());
                match status_of(&entity) {
                    Some(WorkflowStatus::Running) => {
                        self.try_resume_confirmed_task(&entity).await?;
                    }
                    Some(WorkflowStatus::Cancelled) => {
                        self.try_cancel_pending_action(&entity).await?;
                    }
                    _ => {}
                }
            }
        }
    }

    fn remember_task(&mut self, task: Entity) {
        match self
            .known_tasks
            .iter_mut()
            .find(|known| known.id == task.id)
        {
            Some(slot) => *slot = task,
            None => self.known_tasks.push(task),
        }
    }

    async fn process_intent(&mut self, intent: &Entity) -> Result<(), ClientError> {
        if let Some(target) = dangerous_action_of(intent) {
            return self.process_dangerous_intent(intent, target).await;
        }

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
        self.remember_task(task.clone());

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
        self.remember_task(updated_task);

        eprintln!("saai-taskd: intent {} done -> {summary}", intent.id);
        Ok(())
    }

    /// Creates the Task/Action pair for a dangerous intent and stops --
    /// both start (and stay) at `WaitingConfirmation` until an external
    /// write moves the Task to `Running`. Nothing here executes
    /// anything; that's `execute_confirmed_action`'s job, and only
    /// after a live confirmation, never from this function.
    async fn process_dangerous_intent(
        &mut self,
        intent: &Entity,
        target: Uuid,
    ) -> Result<(), ClientError> {
        let short = short_id(target);
        eprintln!(
            "saai-taskd: intent {} is dangerous (delete_entity {target}), pausing for confirmation",
            intent.id
        );

        let task = self
            .conn
            .create_entity(
                &self.space_id,
                TASK_TYPE,
                &format!("Подтвердите: удалить объект {short}"),
                task_properties(intent.id, WorkflowStatus::WaitingConfirmation),
            )
            .await?;
        self.remember_task(task.clone());

        let input = json!({ "target_entity_id": target.to_string() });
        self.conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &format!("Действие: удалить объект {short}"),
                action_properties(
                    task.id,
                    DELETE_ENTITY_ACTION_KIND,
                    WorkflowStatus::WaitingConfirmation,
                    &input,
                    None,
                ),
            )
            .await?;

        eprintln!("saai-taskd: task {} waiting for confirmation", task.id);
        Ok(())
    }

    /// A Task just became (or already was, at reconcile time) `Running`
    /// -- but that alone doesn't mean it needs executing: Change 2's
    /// own non-dangerous Tasks are created `Running` too, and this
    /// daemon's own writes echo back through its own `Subscribe`
    /// stream. The Action's *own* current status is what actually
    /// decides: only `WaitingConfirmation` means "a human just
    /// confirmed this and it hasn't run yet." Returns whether it
    /// actually executed something, purely so callers can report a
    /// count.
    async fn try_resume_confirmed_task(&mut self, task: &Entity) -> Result<bool, ClientError> {
        let actions: Vec<Entity> = self
            .conn
            .list_entities(&self.space_id)
            .await?
            .into_iter()
            .filter(|entity| entity.entity_type == ACTION_TYPE)
            .collect();
        let Some(action) = find_action_for_task(&actions, task.id) else {
            return Ok(false);
        };
        if status_of(action) != Some(WorkflowStatus::WaitingConfirmation) {
            return Ok(false);
        }
        self.execute_confirmed_action(task, action).await?;
        Ok(true)
    }

    /// A Task just became (or already was) `Cancelled` -- mirrors
    /// `try_resume_confirmed_task`'s idempotency shape exactly, just
    /// for the decline side: only a `WaitingConfirmation` Action is
    /// still unexecuted and worth marking `Cancelled` too, purely so
    /// the audit trail doesn't leave it stuck looking like it's still
    /// pending forever. Never runs anything.
    async fn try_cancel_pending_action(&mut self, task: &Entity) -> Result<(), ClientError> {
        let actions: Vec<Entity> = self
            .conn
            .list_entities(&self.space_id)
            .await?
            .into_iter()
            .filter(|entity| entity.entity_type == ACTION_TYPE)
            .collect();
        let Some(action) = find_action_for_task(&actions, task.id) else {
            return Ok(());
        };
        if status_of(action) != Some(WorkflowStatus::WaitingConfirmation) {
            return Ok(());
        }
        let kind = action
            .properties
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or(DELETE_ENTITY_ACTION_KIND)
            .to_string();
        let input = action.properties.get("input").cloned().unwrap_or(json!({}));
        self.conn
            .update_entity(
                action,
                action_properties(task.id, &kind, WorkflowStatus::Cancelled, &input, None),
            )
            .await?;
        eprintln!(
            "saai-taskd: task {} cancelled, action left unexecuted",
            task.id
        );
        Ok(())
    }

    async fn execute_confirmed_action(
        &mut self,
        task: &Entity,
        action: &Entity,
    ) -> Result<(), ClientError> {
        let input = action.properties.get("input").cloned().unwrap_or(json!({}));
        let target: Uuid = input
            .get("target_entity_id")
            .and_then(Value::as_str)
            .and_then(|raw| raw.parse().ok())
            .ok_or_else(|| {
                ClientError::UnexpectedResult(
                    "delete_entity action missing target_entity_id".into(),
                )
            })?;

        eprintln!("saai-taskd: task {} confirmed, deleting {target}", task.id);
        let deleted = self.delete_target_entity(target).await?;
        let summary = if deleted {
            format!("Объект {target} удалён")
        } else {
            format!("Объект {target} уже отсутствовал (удалён ранее)")
        };
        let output = json!({ "target_entity_id": target.to_string(), "deleted": deleted });

        let action = self
            .conn
            .update_entity(
                action,
                action_properties(
                    task.id,
                    DELETE_ENTITY_ACTION_KIND,
                    WorkflowStatus::Done,
                    &input,
                    Some(&output),
                ),
            )
            .await?;

        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &summary,
                result_properties(task.id, action.id, &summary),
            )
            .await?;

        let intent_id = model::intent_id_of(task)
            .ok_or_else(|| ClientError::UnexpectedResult("task missing intent_id".into()))?;
        let mut done_properties = task_properties(intent_id, WorkflowStatus::Done);
        done_properties.insert("result_id".into(), json!(result.id.to_string()));
        let updated_task = self.conn.update_entity(task, done_properties).await?;
        self.remember_task(updated_task);

        eprintln!("saai-taskd: task {} done -> {summary}", task.id);
        Ok(())
    }

    /// `Ok(false)` (not an error) when the target is already gone --
    /// confirming the same Task twice (a duplicate event, or a
    /// reconcile-after-restart re-seeing an already-executed
    /// confirmation) must not fail just because there's nothing left to
    /// delete the second time.
    async fn delete_target_entity(&mut self, target: Uuid) -> Result<bool, ClientError> {
        let entities = self.conn.list_entities(&self.space_id).await?;
        let Some(existing) = entities.into_iter().find(|entity| entity.id == target) else {
            return Ok(false);
        };
        self.conn
            .delete_entity(&self.space_id, existing.id, existing.revision)
            .await?;
        Ok(true)
    }
}

fn short_id(id: Uuid) -> String {
    let full = id.to_string();
    full.split('-').next().unwrap_or(&full).to_string()
}
