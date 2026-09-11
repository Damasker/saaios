//! `saai-taskd`: S09's minimal vertical slice, extended by S10's planner
//! bridge. Watches one space's `saai-entityd` feed for `saaios.intent`
//! entities and turns each into a `saaios.task` -> `saaios.action` ->
//! `saaios.result`, per ADR-030's decision to model this natively on
//! `saai-entity-store` rather than on Platform Track's
//! `policy-engine`/`tool-registry`.
//!
//! Change 3 (ADR-032) built the dangerous path: an intent that selects
//! `delete_entity` pauses at `WaitingConfirmation` instead of running --
//! this daemon never transitions a Task out of that state itself. Only
//! an external write (a live touch on `saai-shell`'s confirmation
//! screen) can move it to `Running`, and this daemon then finishes the
//! chain reactively. A restart re-enters that same reactive wait, per
//! S09's Threat/privacy impact requirement that a resumed Task never
//! auto-executes on a cached decision.
//!
//! S10 Change 2 (ADR-033) replaces the old placeholder `echo` path:
//! every other free-form intent now asks the already-running, already
//! physically-proven `saaios-runtime` what to do (`runtime_bridge`)
//! instead of a hardcoded transform. Whether the resulting Task becomes
//! `Done` immediately or `WaitingConfirmation` is decided purely by
//! whether that response carries a `pending` proposal -- the same
//! confirmation screen, the same touch, the same audit trail as the
//! `delete_entity` path, just a different Action `kind` and a different
//! executor underneath it.
//!
//! Not yet wired into `native-init.c`'s boot sequence (unlike
//! `saai-appd`/`saai-entityd`) -- this Change proves the workflow
//! itself first, run manually, the same "prove it physically before
//! touching boot-critical init" order S06/S07 already followed for
//! their own daemons.

pub mod client;
pub mod model;
pub mod runtime_bridge;

use chrono::Utc;
use client::{ClientError, EntitydConn};
use model::{
    action_properties, dangerous_action_of, find_action_for_task, has_task_for_intent,
    is_schedule_due, result_properties, safe_title, schedule_every_secs, schedule_fire_count,
    schedule_properties, schedule_text, status_of, task_properties, WorkflowStatus, ACTION_TYPE,
    DELETE_ENTITY_ACTION_KIND, INTENT_TYPE, RESULT_TYPE, RUNTIME_ACTION_KIND, SCHEDULE_TYPE,
    TASK_TYPE,
};
use saai_entity_protocol::{Entity, EntitydEvent};
use saai_entity_store::EventPayload;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

/// S10 Change 3 (ADR-036): how often this daemon checks its space's
/// `saaios.schedule` entities for anything due. Five seconds is short
/// enough to physically verify a schedule firing without a multi-minute
/// wait, and cheap enough (one `list_entities` call, no IO if nothing's
/// due) that a tighter production cadence isn't worth the complexity of
/// making it configurable until something actually needs one.
const SCHEDULE_TICK_INTERVAL: Duration = Duration::from_secs(5);

pub struct Daemon {
    conn: EntitydConn,
    space_id: String,
    /// `host:port` of the on-device `saaios-runtime` this daemon bridges
    /// free-form intents to (ADR-033). Not hardcoded -- it's specific to
    /// how this particular device's `saaios-runtime` is reached (USB-NCM
    /// address of a connected host today; a different device or
    /// transport would need a different value).
    runtime_addr: String,
    known_tasks: Vec<Entity>,
}

impl Daemon {
    pub async fn connect(
        socket: &Path,
        space_id: String,
        runtime_addr: String,
    ) -> Result<Self, ClientError> {
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
            runtime_addr,
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
        let mut schedule_tick = tokio::time::interval(SCHEDULE_TICK_INTERVAL);
        // The first `tick()` on a freshly created interval fires
        // immediately -- consume it up front so `run()`'s very first
        // loop iteration doesn't race a real entityd event with a
        // schedule check that has nothing to find yet.
        schedule_tick.tick().await;
        loop {
            tokio::select! {
                event = self.conn.next_event() => {
                    let EntitydEvent::EntityChanged { record } = event? else {
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
                _ = schedule_tick.tick() => {
                    self.evaluate_due_schedules().await?;
                }
            }
        }
    }

    /// S10 Change 3 (ADR-036): lists this space's `saaios.schedule`
    /// entities, and for each one that's due, creates a `saaios.intent`
    /// with its `text` -- identical in shape to what the on-screen
    /// keyboard creates, so `process_intent()` needs no changes at all
    /// to pick it up on the very next loop iteration (delivered back
    /// through the normal `Subscribe` stream, same as any other write).
    /// Firing updates the schedule's own `last_fired_at`/`fire_count`
    /// before creating the Intent, not after -- a crash between the two
    /// undercounts a slow consumer rather than ever double-firing one on
    /// retry.
    async fn evaluate_due_schedules(&mut self) -> Result<usize, ClientError> {
        let now = Utc::now();
        let schedules: Vec<Entity> = self
            .conn
            .list_entities(&self.space_id)
            .await?
            .into_iter()
            .filter(|entity| entity.entity_type == SCHEDULE_TYPE)
            .collect();

        let mut fired = 0;
        for schedule in schedules {
            if !is_schedule_due(&schedule, now) {
                continue;
            }
            let Some(text) = schedule_text(&schedule).map(str::to_string) else {
                continue;
            };
            let fire_count = schedule_fire_count(&schedule) + 1;
            let every_secs = schedule_every_secs(&schedule).unwrap_or(0);
            self.conn
                .update_entity(
                    &schedule,
                    schedule_properties(every_secs, &text, true, Some(now), fire_count),
                )
                .await?;
            let mut intent_properties = serde_json::Map::new();
            intent_properties.insert("text".into(), json!(text));
            intent_properties.insert("schedule_id".into(), json!(schedule.id.to_string()));
            self.conn
                .create_entity(
                    &self.space_id,
                    INTENT_TYPE,
                    &safe_title(&text, "Расписание"),
                    intent_properties,
                )
                .await?;
            eprintln!(
                "saai-taskd: schedule {} fired ({fire_count}) -> intent(\"{text}\")",
                schedule.id
            );
            fired += 1;
        }
        Ok(fired)
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
        self.process_planner_intent(intent, &text).await
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

    /// S10 Change 2 (ADR-033): asks the already-running `saaios-runtime`
    /// what a free-form intent's text means, instead of a hardcoded
    /// transform. The Task starts `Pending` -- neither `Done` nor
    /// `WaitingConfirmation` is known until the runtime actually
    /// answers. A bridge failure (runtime unreachable, malformed
    /// response) fails the Task cleanly rather than propagating an
    /// error out of this function -- turning the model off must not
    /// crash this daemon or block the explicit-Action paths (S10's
    /// Acceptance criteria).
    async fn process_planner_intent(
        &mut self,
        intent: &Entity,
        text: &str,
    ) -> Result<(), ClientError> {
        eprintln!("saai-taskd: intent {} -> planner(\"{text}\")", intent.id);

        let task = self
            .conn
            .create_entity(
                &self.space_id,
                TASK_TYPE,
                &safe_title(&format!("Задача: {text}"), "Задача"),
                task_properties(intent.id, WorkflowStatus::Pending),
            )
            .await?;
        self.remember_task(task.clone());

        let response =
            match runtime_bridge::diagnose(&self.runtime_addr, text, &self.space_id).await {
                Ok(response) => response,
                Err(error) => return self.fail_task(&task, intent.id, &error.to_string()).await,
            };

        if !response.ok {
            let message = response
                .error
                .clone()
                .unwrap_or_else(|| "saaios-runtime returned an error".into());
            return self.fail_task(&task, intent.id, &message).await;
        }

        if let Some(pending) = response.pending.clone() {
            let input = json!({
                "tool": pending.tool,
                "arguments": pending.arguments,
                "call_id": pending.call_id.to_string(),
                "correlation_id": response.correlation_id.map(|id| id.to_string()),
                "session_id": response.session_id.map(|id| id.to_string()),
                "summary": pending.summary,
            });
            self.conn
                .create_entity(
                    &self.space_id,
                    ACTION_TYPE,
                    &safe_title(&format!("Действие: {}", pending.tool), "Действие"),
                    action_properties(
                        task.id,
                        RUNTIME_ACTION_KIND,
                        WorkflowStatus::WaitingConfirmation,
                        &input,
                        None,
                    ),
                )
                .await?;
            let updated_task = self
                .conn
                .update_entity(
                    &task,
                    task_properties(intent.id, WorkflowStatus::WaitingConfirmation),
                )
                .await?;
            self.remember_task(updated_task);
            eprintln!(
                "saai-taskd: task {} waiting for confirmation ({})",
                task.id, pending.tool
            );
            return Ok(());
        }

        // No pending proposal -- saaios-runtime's own policy-engine
        // already judged everything it did along the way safe enough to
        // run without asking, so this Task never needs a native
        // confirmation gate either (ADR-033's answer to Change 1's
        // second question).
        let running_task = self
            .conn
            .update_entity(&task, task_properties(intent.id, WorkflowStatus::Running))
            .await?;

        let summary = response.summary().unwrap_or_default().to_string();
        let action_input = json!({ "text": text });
        let action_output = json!({ "summary": summary });
        let action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                "Действие: ответ модели",
                action_properties(
                    running_task.id,
                    RUNTIME_ACTION_KIND,
                    WorkflowStatus::Done,
                    &action_input,
                    Some(&action_output),
                ),
            )
            .await?;

        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &safe_title(&summary, "Результат"),
                result_properties(running_task.id, action.id, &summary),
            )
            .await?;

        let mut done_properties = task_properties(intent.id, WorkflowStatus::Done);
        done_properties.insert("result_id".into(), json!(result.id.to_string()));
        let updated_task = self
            .conn
            .update_entity(&running_task, done_properties)
            .await?;
        self.remember_task(updated_task);

        eprintln!("saai-taskd: intent {} done -> {summary}", intent.id);
        Ok(())
    }

    async fn fail_task(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        message: &str,
    ) -> Result<(), ClientError> {
        eprintln!("saai-taskd: task {} failed: {message}", task.id);
        let mut failed_properties = task_properties(intent_id, WorkflowStatus::Failed);
        failed_properties.insert("error".into(), json!(message));
        let updated_task = self.conn.update_entity(task, failed_properties).await?;
        self.remember_task(updated_task);
        Ok(())
    }

    /// A Task just became (or already was, at reconcile time) `Running`
    /// -- but that alone doesn't mean it needs executing: non-dangerous
    /// planner Tasks are created `Running` too (right before completing
    /// on their own), and this daemon's own writes echo back through its
    /// own `Subscribe` stream. The Action's *own* current status is what
    /// actually decides: only `WaitingConfirmation` means "a human just
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
    /// pending forever. For a `saaios_runtime_tool` Action, also tells
    /// `saaios-runtime` itself (`confirmed: false`) so its own pending
    /// state doesn't dangle -- physically confirmed in ADR-033's spike
    /// to produce a clean `"user cancelled"` outcome. Never runs
    /// anything.
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

        if kind == RUNTIME_ACTION_KIND {
            if let (Some(call_id), Some(correlation_id)) = (
                uuid_field(&input, "call_id"),
                uuid_field(&input, "correlation_id"),
            ) {
                let session_id = uuid_field(&input, "session_id");
                let tool = input
                    .get("tool")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let arguments = input.get("arguments").cloned().unwrap_or(json!({}));
                // Best-effort: saaios-runtime's own pending state is
                // process-local and already tolerant of a client that
                // never confirms at all -- if this call fails, the
                // native Cancelled status below is still the source of
                // truth `saai-shell`/`saai-taskd` themselves rely on.
                let _ = runtime_bridge::confirm(
                    &self.runtime_addr,
                    correlation_id,
                    session_id,
                    call_id,
                    tool,
                    arguments,
                    false,
                )
                .await;
            }
        }

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
        let kind = action
            .properties
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        match kind.as_str() {
            DELETE_ENTITY_ACTION_KIND => self.execute_delete_entity_action(task, action).await,
            RUNTIME_ACTION_KIND => self.execute_runtime_action(task, action).await,
            other => Err(ClientError::UnexpectedResult(format!(
                "confirmed action has unknown kind {other:?}"
            ))),
        }
    }

    async fn execute_delete_entity_action(
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

        self.finish_task(task, action.id, &summary).await
    }

    /// S10 Change 2: the confirmed side of the planner bridge. Resumes
    /// exactly the `call_id` a prior `process_planner_intent()` left
    /// pending, via `{"op":"confirm", ..., "confirmed": true}`.
    async fn execute_runtime_action(
        &mut self,
        task: &Entity,
        action: &Entity,
    ) -> Result<(), ClientError> {
        let input = action.properties.get("input").cloned().unwrap_or(json!({}));
        let tool = input
            .get("tool")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let arguments = input.get("arguments").cloned().unwrap_or(json!({}));

        let (Some(call_id), Some(correlation_id)) = (
            uuid_field(&input, "call_id"),
            uuid_field(&input, "correlation_id"),
        ) else {
            return self
                .fail_confirmed_action(task, action, &input, "missing call_id/correlation_id")
                .await;
        };
        let session_id = uuid_field(&input, "session_id");

        eprintln!(
            "saai-taskd: task {} confirmed, asking saaios-runtime to run {tool}",
            task.id
        );

        let response = match runtime_bridge::confirm(
            &self.runtime_addr,
            correlation_id,
            session_id,
            call_id,
            &tool,
            arguments,
            true,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => {
                return self
                    .fail_confirmed_action(task, action, &input, &error.to_string())
                    .await
            }
        };

        let (ok, output, error_message) = match response.tool_output() {
            Some((ok, output, error)) => (ok, output.clone(), error.map(str::to_string)),
            None => (response.ok, Value::Null, response.error.clone()),
        };

        let summary = if ok {
            format!("Выполнено: {tool}")
        } else {
            format!(
                "Не выполнено: {tool} ({})",
                error_message.as_deref().unwrap_or("неизвестная ошибка")
            )
        };
        let action_output = json!({ "ok": ok, "output": output, "error": error_message });

        let action = self
            .conn
            .update_entity(
                action,
                action_properties(
                    task.id,
                    RUNTIME_ACTION_KIND,
                    WorkflowStatus::Done,
                    &input,
                    Some(&action_output),
                ),
            )
            .await?;

        self.finish_task(task, action.id, &summary).await
    }

    async fn fail_confirmed_action(
        &mut self,
        task: &Entity,
        action: &Entity,
        input: &Value,
        message: &str,
    ) -> Result<(), ClientError> {
        eprintln!(
            "saai-taskd: task {} confirmed action failed: {message}",
            task.id
        );
        let action_output = json!({ "ok": false, "error": message });
        self.conn
            .update_entity(
                action,
                action_properties(
                    task.id,
                    RUNTIME_ACTION_KIND,
                    WorkflowStatus::Failed,
                    input,
                    Some(&action_output),
                ),
            )
            .await?;

        let intent_id = model::intent_id_of(task)
            .ok_or_else(|| ClientError::UnexpectedResult("task missing intent_id".into()))?;
        let mut failed_properties = task_properties(intent_id, WorkflowStatus::Failed);
        failed_properties.insert("error".into(), json!(message));
        let updated_task = self.conn.update_entity(task, failed_properties).await?;
        self.remember_task(updated_task);
        Ok(())
    }

    /// Shared tail of both confirmed-Action executors: create the
    /// Result, move the Task to `Done`, remember it. `action_id` is
    /// already the post-update entity's id (unchanged by the update,
    /// but taken explicitly so callers pass the entity they just got
    /// back from `update_entity`, not the pre-update one).
    async fn finish_task(
        &mut self,
        task: &Entity,
        action_id: Uuid,
        summary: &str,
    ) -> Result<(), ClientError> {
        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &safe_title(summary, "Результат"),
                result_properties(task.id, action_id, summary),
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

fn uuid_field(value: &Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse().ok())
}

fn short_id(id: Uuid) -> String {
    let full = id.to_string();
    full.split('-').next().unwrap_or(&full).to_string()
}
