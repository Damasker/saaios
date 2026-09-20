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
//! `native-init.c` starts it from `/data/saaios/system/saai-taskd`
//! after entityd and `saaios-runtime` (ADR-233/234). Missing binary is
//! skipped, not a boot failure. `--space` is the boot selection;
//! `SelectionChanged` retargets the watch (ADR-235) so Home/personal
//! intents are not stalled on a Work-only daemon.

pub mod client;
pub mod graph;
pub mod model;
pub mod runtime_bridge;
pub mod scheduler;

use chrono::Utc;
use client::{ClientError, EntitydConn};
use intent_resolution::{
    resolve_deterministic, AllowedContext, ClarificationResolution, IntentInput, IntentSource,
    ResolutionOutcome, ResolveAttempt, PLAN_PROPERTY, SEMANTIC_ACTION_PROPERTY, SOURCE_PROPERTY,
};
use model::{
    action_properties, dangerous_action_of, evidence_from_fresh_rows, find_action_for_task,
    has_open_task_for_intent, has_task_for_intent, intent_id_of, is_schedule_due, result_id_of,
    result_properties, safe_title, schedule_every_secs, schedule_fire_count, schedule_properties,
    schedule_text, should_retry_failed_task, status_after_verification, status_of, task_properties,
    task_properties_after_result, verification_key_of, with_depends_on, ObservationEvidence,
    WorkflowStatus, ACTION_TYPE, DELETE_ENTITY_ACTION_KIND, INTENT_TYPE, NOTIFICATION_TYPE,
    PROPOSAL_ID_PROPERTY, RESULT_TYPE, RUNTIME_ACTION_KIND, SCHEDULE_TYPE, SEMANTIC_ACTION_KIND,
    TASK_TYPE,
};
use saai_entity_protocol::{
    Entity, EntitydEvent, RELATION_EXECUTES, RELATION_PRODUCES, RELATION_REALIZES,
};
use saai_entity_store::EventPayload;
use saai_object_actions::{display_inspect_spec, ObjectActionRegistry};
use scheduler::{admit_frontier, derive_ready_set, mutating_in_flight, MAX_MUTATING_IN_FLIGHT};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
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

    pub fn watch_space(&self) -> &str {
        &self.space_id
    }

    /// ADR-235: entityd already broadcasts `SelectionChanged`. Follow it
    /// instead of staying on the `--space` passed at exec. Same-space
    /// events are a no-op. Switching reloads this space's tasks and
    /// reconciles intents/confirmed work that arrived while we watched
    /// somewhere else. In-flight work in the previous space stays there.
    pub async fn follow_selected_space(&mut self, space_id: String) -> Result<bool, ClientError> {
        if space_id == self.space_id {
            return Ok(false);
        }
        eprintln!("saai-taskd: follow space {} -> {}", self.space_id, space_id);
        self.space_id = space_id;
        self.known_tasks = self
            .conn
            .list_entities(&self.space_id)
            .await?
            .into_iter()
            .filter(|entity| entity.entity_type == TASK_TYPE)
            .collect();
        self.reconcile_existing_intents().await?;
        self.reconcile_confirmed_tasks().await?;
        self.dispatch_ready().await?;
        Ok(true)
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
        self.settle_verifying_tasks().await?;
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
                    match event? {
                        EntitydEvent::SelectionChanged { selection } => {
                            self.follow_selected_space(selection.space_id).await?;
                        }
                        EntitydEvent::EntityChanged { record } => {
                            if record.space_id != self.space_id {
                                continue;
                            }
                            let entity = match record.payload {
                                EventPayload::EntityCreated { entity }
                                | EventPayload::EntityUpdated { entity } => entity,
                                EventPayload::SpaceCreated { .. }
                                | EventPayload::EntityDeleted { .. } => continue,
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
                                    Some(WorkflowStatus::Failed) => {
                                        self.try_retry_failed_task(&entity).await?;
                                        self.dispatch_ready().await?;
                                    }
                                    Some(WorkflowStatus::Verifying) => {
                                        self.settle_verifying_task(&entity).await?;
                                    }
                                    Some(WorkflowStatus::Done) | Some(WorkflowStatus::Pending) => {
                                        self.dispatch_ready().await?;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        EntitydEvent::RelationshipChanged { .. } => continue,
                    }
                }
                _ = schedule_tick.tick() => {
                    self.evaluate_due_schedules().await?;
                    self.settle_verifying_tasks().await?;
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
            intent_properties.insert(
                SOURCE_PROPERTY.into(),
                json!(IntentSource::Schedule.as_str()),
            );
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
            Some(slot) if task.revision < slot.revision => {}
            Some(slot) => *slot = task,
            None => self.known_tasks.push(task),
        }
    }

    /// SOM lineage is written beside legacy `intent_id`/`task_id`
    /// properties. A failed relation must not roll back the entity:
    /// the two writes are sequential, not a transaction.
    async fn record_lineage(&mut self, source: Uuid, target: Uuid, relation_type: &str) {
        if let Err(error) = self
            .conn
            .create_relationship(source, target, relation_type)
            .await
        {
            eprintln!("saai-taskd: lineage {relation_type} not recorded: {error}");
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
        let input = IntentInput::from_entity(intent);
        let allowed = allowed_context_for(&input);
        log_irab_context(intent.id, &input);
        match resolve_deterministic(&input, &allowed) {
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Action(action),
                trace,
            } => {
                eprintln!(
                    "IRAB: intent={} source={:?} focused_object={:?} deterministic=true outcome=action semantic_action={} target_source={:?} action_source={:?}",
                    intent.id,
                    input.source,
                    input.context.focused_ref(),
                    action.action_id,
                    trace.target_source,
                    trace.action_source
                );
                self.process_resolved_action(intent, &text, &action).await
            }
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Clarification(clarification),
                ..
            } => {
                eprintln!(
                    "IRAB: intent={} outcome=clarification options={}",
                    intent.id,
                    clarification.options.len()
                );
                self.process_clarification(intent, &text, clarification)
                    .await
            }
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Unsupported(unsupported),
                ..
            } => {
                eprintln!(
                    "IRAB: intent={} outcome=unsupported reason={}",
                    intent.id, unsupported.reason
                );
                self.process_unsupported(intent, &text, &unsupported.reason)
                    .await
            }
            ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Plan(plan),
                ..
            } => {
                eprintln!("IRAB: intent={} outcome=plan goal={}", intent.id, plan.goal);
                self.process_plan_intent(intent, &plan.goal).await
            }
            ResolveAttempt::NeedsModel
            | ResolveAttempt::Resolved {
                outcome: ResolutionOutcome::Answer(_),
                ..
            } => {
                eprintln!(
                    "IRAB fallback: legacy_runtime_diagnose intent={}",
                    intent.id
                );
                self.process_planner_intent(intent, &text).await
            }
        }
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
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;

        let input = json!({ "target_entity_id": target.to_string() });
        let action = self
            .conn
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
        self.record_lineage(action.id, task.id, RELATION_EXECUTES)
            .await;

        eprintln!("saai-taskd: task {} waiting for confirmation", task.id);
        Ok(())
    }

    /// Whether `action_id` is safe to auto-complete without a live
    /// confirmation. Looks the id back up in the same OAM registry
    /// `allowed_context_for` already built the resolution against
    /// (`builtin_oam()`, cheap and stateless -- matches that function's
    /// own style, not a new caching concern to get right). `None` (the
    /// spec vanished, or was never real) is treated as "cannot confirm
    /// this is safe," the same fail-closed direction the check itself
    /// takes for a spec found with `requires_confirmation: true` --
    /// never fail-open into auto-completing something this function
    /// cannot vouch for.
    fn action_requires_confirmation(action_id: &str) -> bool {
        builtin_oam()
            .spec_for(action_id)
            .is_none_or(|spec| spec.requires_confirmation)
    }

    /// Creates the Task/Action pair for a semantic action whose spec
    /// requires confirmation, and stops -- same shape as
    /// `process_dangerous_intent`'s own delete-entity path, generalized
    /// to any OAM action id. Both start (and stay) at
    /// `WaitingConfirmation`; nothing here executes anything or
    /// fabricates a `Result`. A real "confirm and dispatch" path for
    /// semantic actions does not exist yet (only `delete_entity`'s own
    /// hand-built confirm handler does) -- this function's job is only
    /// to stop the false "Done" from ever being written, not to build
    /// that dispatcher.
    async fn process_action_requiring_confirmation(
        &mut self,
        intent: &Entity,
        action: &intent_resolution::ActionResolution,
    ) -> Result<(), ClientError> {
        let title = format!(
            "Подтвердите: {} → {}",
            target_label(&action.target),
            action.action_id
        );
        eprintln!(
            "saai-taskd: intent {} resolved to {} which requires confirmation, pausing",
            intent.id, action.action_id
        );
        let task = self
            .conn
            .create_entity(
                &self.space_id,
                TASK_TYPE,
                &safe_title(&title, "Задача"),
                task_properties(intent.id, WorkflowStatus::WaitingConfirmation),
            )
            .await?;
        self.remember_task(task.clone());
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;
        let action_input = json!({
            "semantic_action_id": action.action_id,
            "target": action.target,
            "parameters": action.parameters,
            "target_revision": action.target_revision,
        });
        let stored_action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &safe_title(&format!("Действие: {}", action.action_id), "Действие"),
                action_properties(
                    task.id,
                    SEMANTIC_ACTION_KIND,
                    WorkflowStatus::WaitingConfirmation,
                    &action_input,
                    None,
                ),
            )
            .await?;
        self.record_lineage(stored_action.id, task.id, RELATION_EXECUTES)
            .await;
        eprintln!("saai-taskd: task {} waiting for confirmation", task.id);
        Ok(())
    }

    async fn process_resolved_action(
        &mut self,
        intent: &Entity,
        text: &str,
        action: &intent_resolution::ActionResolution,
    ) -> Result<(), ClientError> {
        if Self::action_requires_confirmation(&action.action_id) {
            return self
                .process_action_requiring_confirmation(intent, action)
                .await;
        }
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
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;
        let running_task = self
            .conn
            .update_entity(&task, task_properties(intent.id, WorkflowStatus::Running))
            .await?;
        let summary = format!(
            "Распознано: {} → {}",
            target_label(&action.target),
            action.action_id
        );
        let action_input = json!({
            "semantic_action_id": action.action_id,
            "target": action.target,
            "parameters": action.parameters,
            "target_revision": action.target_revision,
            "text": text,
        });
        let action_output = json!({ "summary": summary, "executed": false });
        let stored_action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &safe_title(&format!("Действие: {}", action.action_id), "Действие"),
                action_properties(
                    running_task.id,
                    SEMANTIC_ACTION_KIND,
                    WorkflowStatus::Done,
                    &action_input,
                    Some(&action_output),
                ),
            )
            .await?;
        self.record_lineage(stored_action.id, running_task.id, RELATION_EXECUTES)
            .await;
        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &safe_title(&summary, "Результат"),
                result_properties(running_task.id, stored_action.id, &summary),
            )
            .await?;
        self.record_lineage(stored_action.id, result.id, RELATION_PRODUCES)
            .await;
        self.advance_task_after_result(&running_task, intent.id, result.id)
            .await?;
        eprintln!(
            "saai-taskd: task {} irab-resolved {} without runtime diagnose",
            running_task.id, action.action_id
        );
        Ok(())
    }

    async fn process_clarification(
        &mut self,
        intent: &Entity,
        text: &str,
        clarification: ClarificationResolution,
    ) -> Result<(), ClientError> {
        let mut properties = task_properties(intent.id, WorkflowStatus::WaitingClarification);
        properties.insert(
            "clarification".into(),
            serde_json::to_value(&clarification).unwrap_or(json!({})),
        );
        properties.insert("text".into(), json!(text));
        let task = self
            .conn
            .create_entity(
                &self.space_id,
                TASK_TYPE,
                &safe_title(&clarification.question, "Уточнение"),
                properties,
            )
            .await?;
        self.remember_task(task.clone());
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;
        eprintln!("saai-taskd: task {} waiting for clarification", task.id);
        Ok(())
    }

    async fn process_unsupported(
        &mut self,
        intent: &Entity,
        text: &str,
        reason: &str,
    ) -> Result<(), ClientError> {
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
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;
        self.fail_task(&task, intent.id, reason).await
    }

    /// ADR-238: a structured PlanProposal becomes a DAG of Tasks, never
    /// one diagnose. Invalid graphs fail one Task so the Intent is not
    /// left empty. Confirmation stays per Action; the plan does not
    /// bulk-allow.
    async fn process_plan_intent(
        &mut self,
        intent: &Entity,
        goal: &str,
    ) -> Result<(), ClientError> {
        let Some(raw) = intent.properties.get(PLAN_PROPERTY).cloned() else {
            return self
                .process_unsupported(intent, goal, "plan outcome without a plan body")
                .await;
        };
        let proposal: graph::PlanProposal = match serde_json::from_value(raw) {
            Ok(proposal) => proposal,
            Err(error) => {
                return self
                    .process_unsupported(intent, goal, &format!("malformed plan: {error}"))
                    .await;
            }
        };
        let bound = match graph::bind_plan(&proposal) {
            Ok(bound) => bound,
            Err(error) => {
                return self
                    .process_unsupported(intent, goal, &error.to_string())
                    .await;
            }
        };
        eprintln!(
            "IRAB: intent={} persist plan tasks={}",
            intent.id,
            bound.len()
        );
        let mut ids = HashMap::new();
        for step in &bound {
            let deps = graph::remap_depends_on(step, &ids);
            let mut properties =
                with_depends_on(task_properties(intent.id, WorkflowStatus::Pending), &deps);
            properties.insert(PROPOSAL_ID_PROPERTY.into(), json!(step.proposal_id));
            if let Some(action_id) = &step.action_id {
                properties.insert(SEMANTIC_ACTION_PROPERTY.into(), json!(action_id));
            }
            if let Some(target) = &step.target {
                properties.insert("target".into(), target.clone());
            }
            if !step.parameters.is_null() {
                properties.insert("parameters".into(), step.parameters.clone());
            }
            let task = self
                .conn
                .create_entity(
                    &self.space_id,
                    TASK_TYPE,
                    &safe_title(&step.title, "Задача"),
                    properties,
                )
                .await?;
            ids.insert(step.proposal_id.clone(), task.id);
            self.remember_task(task.clone());
            self.record_lineage(task.id, intent.id, RELATION_REALIZES)
                .await;
        }
        self.dispatch_ready().await?;
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
        self.record_lineage(task.id, intent.id, RELATION_REALIZES)
            .await;
        self.dispatch_ready().await?;
        Ok(())
    }

    /// ADR-237: Ready is derived; this is the only place that starts
    /// planner work. `WaitingConfirmation` is never admitted.
    /// Mutating in-flight stays at 1.
    pub async fn dispatch_ready(&mut self) -> Result<usize, ClientError> {
        let mut started = 0;
        loop {
            let ready = derive_ready_set(&self.known_tasks);
            let in_flight = mutating_in_flight(&self.known_tasks);
            let Some(id) = admit_frontier(&ready, in_flight, MAX_MUTATING_IN_FLIGHT)
                .into_iter()
                .next()
            else {
                break;
            };
            let Some(task) = self
                .known_tasks
                .iter()
                .find(|known| known.id == id)
                .cloned()
            else {
                break;
            };
            if status_of(&task) != Some(WorkflowStatus::Pending) {
                break;
            }
            eprintln!(
                "saai-taskd: admit {} ready={} in_flight={}",
                task.id,
                ready.len(),
                in_flight
            );
            let _ = std::io::Write::flush(&mut std::io::stderr());
            self.start_ready_planner_task(&task).await?;
            started += 1;
            let still_pending = self
                .known_tasks
                .iter()
                .find(|known| known.id == id)
                .and_then(status_of)
                == Some(WorkflowStatus::Pending);
            if still_pending {
                break;
            }
        }
        Ok(started)
    }

    async fn start_ready_plan_step(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        action_id: &str,
    ) -> Result<(), ClientError> {
        let Some(target) = task
            .properties
            .get("target")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
        else {
            return self
                .fail_task(task, intent_id, "plan step missing target")
                .await;
        };
        let parameters = task
            .properties
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let action = intent_resolution::ActionResolution {
            target,
            action_id: action_id.to_string(),
            parameters,
            target_revision: None,
        };
        if Self::action_requires_confirmation(action_id) {
            return self.gate_existing_task(task, intent_id, &action).await;
        }
        self.complete_existing_semantic_task(task, intent_id, &action)
            .await
    }

    async fn gate_existing_task(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        action: &intent_resolution::ActionResolution,
    ) -> Result<(), ClientError> {
        let action_input = json!({
            "semantic_action_id": action.action_id,
            "target": action.target,
            "parameters": action.parameters,
            "target_revision": action.target_revision,
        });
        let stored_action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &safe_title(&format!("Действие: {}", action.action_id), "Действие"),
                action_properties(
                    task.id,
                    SEMANTIC_ACTION_KIND,
                    WorkflowStatus::WaitingConfirmation,
                    &action_input,
                    None,
                ),
            )
            .await?;
        self.record_lineage(stored_action.id, task.id, RELATION_EXECUTES)
            .await;
        let updated = self
            .conn
            .update_entity(
                task,
                task_properties(intent_id, WorkflowStatus::WaitingConfirmation),
            )
            .await?;
        self.remember_task(updated);
        eprintln!(
            "saai-taskd: task {} plan step waiting for confirmation ({})",
            task.id, action.action_id
        );
        Ok(())
    }

    async fn complete_existing_semantic_task(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        action: &intent_resolution::ActionResolution,
    ) -> Result<(), ClientError> {
        let running_task = self
            .conn
            .update_entity(task, task_properties(intent_id, WorkflowStatus::Running))
            .await?;
        let summary = format!(
            "Распознано: {} → {}",
            target_label(&action.target),
            action.action_id
        );
        let action_input = json!({
            "semantic_action_id": action.action_id,
            "target": action.target,
            "parameters": action.parameters,
            "target_revision": action.target_revision,
        });
        let action_output = json!({ "summary": summary, "executed": false });
        let stored_action = self
            .conn
            .create_entity(
                &self.space_id,
                ACTION_TYPE,
                &safe_title(&format!("Действие: {}", action.action_id), "Действие"),
                action_properties(
                    running_task.id,
                    SEMANTIC_ACTION_KIND,
                    WorkflowStatus::Done,
                    &action_input,
                    Some(&action_output),
                ),
            )
            .await?;
        self.record_lineage(stored_action.id, running_task.id, RELATION_EXECUTES)
            .await;
        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &safe_title(&summary, "Результат"),
                result_properties(running_task.id, stored_action.id, &summary),
            )
            .await?;
        self.record_lineage(stored_action.id, result.id, RELATION_PRODUCES)
            .await;
        self.advance_task_after_result(&running_task, intent_id, result.id)
            .await?;
        eprintln!(
            "saai-taskd: task {} plan step {} verifying",
            running_task.id, action.action_id
        );
        Ok(())
    }

    async fn start_ready_planner_task(&mut self, task: &Entity) -> Result<(), ClientError> {
        let Some(intent_id) = intent_id_of(task) else {
            return self
                .fail_task(task, Uuid::nil(), "task missing intent_id")
                .await;
        };
        if let Some(action_id) = task
            .properties
            .get(SEMANTIC_ACTION_PROPERTY)
            .and_then(Value::as_str)
            .map(str::to_string)
        {
            return self
                .start_ready_plan_step(task, intent_id, &action_id)
                .await;
        }
        let entities = self.conn.list_entities(&self.space_id).await?;
        let actions: Vec<Entity> = entities
            .iter()
            .filter(|entity| entity.entity_type == ACTION_TYPE)
            .cloned()
            .collect();
        if find_action_for_task(&actions, task.id).is_some() {
            return Ok(());
        }
        let Some(intent) = entities
            .iter()
            .find(|entity| entity.entity_type == INTENT_TYPE && entity.id == intent_id)
        else {
            return self
                .fail_task(task, intent_id, "intent gone before dispatch")
                .await;
        };
        let text = intent
            .properties
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or(&intent.title)
            .to_string();

        let response =
            match runtime_bridge::diagnose(&self.runtime_addr, &text, &self.space_id).await {
                Ok(response) => response,
                Err(error) => {
                    return self.fail_bridge_task(task, intent_id, &error).await;
                }
            };

        if !response.ok {
            let message = response
                .error
                .clone()
                .unwrap_or_else(|| "saaios-runtime returned an error".into());
            return self.fail_runtime_message(task, intent_id, &message).await;
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
            let action = self
                .conn
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
            self.record_lineage(action.id, task.id, RELATION_EXECUTES)
                .await;
            let updated_task = self
                .conn
                .update_entity(
                    task,
                    task_properties(intent_id, WorkflowStatus::WaitingConfirmation),
                )
                .await?;
            self.remember_task(updated_task);
            eprintln!(
                "saai-taskd: task {} waiting for confirmation ({})",
                task.id, pending.tool
            );
            return Ok(());
        }

        let running_task = self
            .conn
            .update_entity(task, task_properties(intent_id, WorkflowStatus::Running))
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
        self.record_lineage(action.id, running_task.id, RELATION_EXECUTES)
            .await;

        let result = self
            .conn
            .create_entity(
                &self.space_id,
                RESULT_TYPE,
                &safe_title(&summary, "Результат"),
                result_properties(running_task.id, action.id, &summary),
            )
            .await?;
        self.record_lineage(action.id, result.id, RELATION_PRODUCES)
            .await;

        self.advance_task_after_result(&running_task, intent_id, result.id)
            .await?;

        eprintln!("saai-taskd: intent {intent_id} verifying -> {summary}");
        Ok(())
    }

    /// ADR-089 (HIA-19's offline/degraded review): a failed Task used
    /// to be visible only by reading the entity store directly --
    /// `saai-shell`'s "Входящие" only ever listed `waiting_
    /// confirmation` Tasks, so an intent that failed (most commonly
    /// because `saaios-runtime` isn't reachable -- ADR-033's own doc
    /// comment already admits it usually lives on a USB-NCM-tethered
    /// host, not the device itself) could vanish without a trace.
    /// Best-effort on purpose -- see this method's own doc comment.
    async fn notify_task_failed(&mut self, task_title: &str, message: &str) {
        let mut properties = Map::new();
        properties.insert("body".into(), json!(message));
        properties.insert("kind".into(), json!("task_failed"));
        if let Err(error) = self
            .conn
            .create_entity(&self.space_id, NOTIFICATION_TYPE, task_title, properties)
            .await
        {
            eprintln!("saai-taskd: failed to create failure notification: {error}");
        }
    }

    async fn fail_bridge_task(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        error: &runtime_bridge::BridgeError,
    ) -> Result<(), ClientError> {
        let kind = if error.is_timeout() {
            Some("timeout")
        } else if matches!(error, runtime_bridge::BridgeError::Connect { .. }) {
            Some("unreachable")
        } else {
            None
        };
        self.fail_task_with(
            task,
            intent_id,
            &error.to_string(),
            error.is_retryable(),
            kind,
        )
        .await
    }

    async fn fail_runtime_message(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        message: &str,
    ) -> Result<(), ClientError> {
        let timeout = runtime_bridge::runtime_error_is_timeout(message);
        self.fail_task_with(
            task,
            intent_id,
            message,
            timeout,
            if timeout { Some("timeout") } else { None },
        )
        .await
    }

    async fn fail_task(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        message: &str,
    ) -> Result<(), ClientError> {
        self.fail_task_with(task, intent_id, message, false, None)
            .await
    }

    async fn fail_task_with(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        message: &str,
        retryable: bool,
        error_kind: Option<&str>,
    ) -> Result<(), ClientError> {
        eprintln!("saai-taskd: task {} failed: {message}", task.id);
        let _ = std::io::Write::flush(&mut std::io::stderr());
        // Best-effort, deliberately not `?` -- a hiccup creating the
        // notification must never turn this Task's own already-
        // durable `Failed` write below into a daemon-crashing error
        // one layer up (`process_intent`'s caller can `std::process::
        // exit(1)` on a propagated error at startup reconcile).
        self.notify_task_failed(&task.title, message).await;
        let mut failed_properties = task_properties(intent_id, WorkflowStatus::Failed);
        failed_properties.insert("error".into(), json!(message));
        if retryable {
            failed_properties.insert("retryable".into(), json!(true));
        }
        if let Some(kind) = error_kind {
            failed_properties.insert("error_kind".into(), json!(kind));
        }
        let updated_task = self.conn.update_entity(task, failed_properties).await?;
        self.remember_task(updated_task);
        Ok(())
    }

    /// ADR-236: a retryable Failed Task whose `retry_requested` flag was
    /// set (Object View later; tests/socket now) gets a sibling Task on
    /// the same Intent. Failed stays Failed. No auto-retry of mutating
    /// Actions (ADR-121).
    async fn try_retry_failed_task(&mut self, task: &Entity) -> Result<bool, ClientError> {
        if !should_retry_failed_task(task) {
            return Ok(false);
        }
        let Some(intent_id) = intent_id_of(task) else {
            return Ok(false);
        };
        if has_open_task_for_intent(&self.known_tasks, intent_id) {
            return Ok(false);
        }
        let mut consumed = task.properties.clone();
        consumed.insert("retry_requested".into(), json!(false));
        let updated = self.conn.update_entity(task, consumed).await?;
        self.remember_task(updated);
        let entities = self.conn.list_entities(&self.space_id).await?;
        let Some(intent) = entities
            .into_iter()
            .find(|entity| entity.entity_type == INTENT_TYPE && entity.id == intent_id)
        else {
            eprintln!("saai-taskd: retry skipped, intent {intent_id} gone");
            return Ok(false);
        };
        eprintln!("saai-taskd: retry task {} intent {intent_id}", task.id);
        let _ = std::io::Write::flush(&mut std::io::stderr());
        self.process_intent(&intent).await?;
        Ok(true)
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

        let response = match runtime_bridge::confirm_as_worker(
            &self.runtime_addr,
            correlation_id,
            session_id,
            call_id,
            &tool,
            arguments,
            true,
            Some(task.id),
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
        self.notify_task_failed(&task.title, message).await;
        let mut failed_properties = task_properties(intent_id, WorkflowStatus::Failed);
        failed_properties.insert("error".into(), json!(message));
        let updated_task = self.conn.update_entity(task, failed_properties).await?;
        self.remember_task(updated_task);
        Ok(())
    }

    /// Shared tail of both confirmed-Action executors: create the
    /// Result, move the Task to `Verifying`, settle if Fresh evidence
    /// already matches. Worker "ok" is never Done (ADR-259).
    /// `action_id` is already the post-update entity's id (unchanged by
    /// the update, but taken explicitly so callers pass the entity they
    /// just got back from `update_entity`, not the pre-update one).
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
        self.record_lineage(action_id, result.id, RELATION_PRODUCES)
            .await;

        let intent_id = model::intent_id_of(task)
            .ok_or_else(|| ClientError::UnexpectedResult("task missing intent_id".into()))?;
        self.advance_task_after_result(task, intent_id, result.id)
            .await?;

        eprintln!("saai-taskd: task {} verifying -> {summary}", task.id);
        Ok(())
    }

    /// Running → Verifying (durable). Done/Failed only from a Fresh
    /// matching Observation. No evidence at Result time stays Verifying.
    async fn advance_task_after_result(
        &mut self,
        task: &Entity,
        intent_id: Uuid,
        result_id: Uuid,
    ) -> Result<(), ClientError> {
        let verifying_properties =
            task_properties_after_result(task, intent_id, result_id, WorkflowStatus::Verifying);
        let verifying = if status_of(task) == Some(WorkflowStatus::Verifying) {
            task.clone()
        } else {
            let updated = self.conn.update_entity(task, verifying_properties).await?;
            self.remember_task(updated.clone());
            updated
        };
        self.settle_verifying_task(&verifying).await?;
        Ok(())
    }

    async fn live_evidence(&self, key: Option<&str>) -> Option<ObservationEvidence> {
        let key = key?;
        match runtime_bridge::status(&self.runtime_addr).await {
            Ok(response) => evidence_from_fresh_rows(
                key,
                response
                    .observations()
                    .iter()
                    .map(|row| (row.key.as_str(), &row.value)),
            ),
            Err(_) => None,
        }
    }

    async fn settle_verifying_tasks(&mut self) -> Result<(), ClientError> {
        let verifying: Vec<Entity> = self
            .known_tasks
            .iter()
            .filter(|task| status_of(task) == Some(WorkflowStatus::Verifying))
            .cloned()
            .collect();
        for task in verifying {
            self.settle_verifying_task(&task).await?;
        }
        Ok(())
    }

    async fn settle_verifying_task(&mut self, task: &Entity) -> Result<(), ClientError> {
        if status_of(task) != Some(WorkflowStatus::Verifying) {
            return Ok(());
        }
        let evidence = self.live_evidence(verification_key_of(task)).await;
        let next = status_after_verification(task, evidence.as_ref());
        if next == WorkflowStatus::Verifying {
            return Ok(());
        }
        let Some(intent_id) = intent_id_of(task) else {
            return Ok(());
        };
        let Some(result_id) = result_id_of(task) else {
            return Ok(());
        };
        let settled = task_properties_after_result(task, intent_id, result_id, next);
        let updated = self.conn.update_entity(task, settled).await?;
        self.remember_task(updated);
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

fn builtin_oam() -> ObjectActionRegistry {
    let mut registry = ObjectActionRegistry::new();
    let _ = registry.register(display_inspect_spec());
    registry
}

fn allowed_context_for(input: &IntentInput) -> AllowedContext {
    let registry = builtin_oam();
    let mut allowed = AllowedContext::default();
    for summary in input
        .context
        .focused_object
        .iter()
        .chain(input.context.selected_objects.iter())
    {
        allowed = allowed.with_object(summary, registry.action_ids_for_type(&summary.entity_type));
    }
    allowed
}

fn log_irab_context(intent_id: Uuid, input: &IntentInput) {
    eprintln!(
        "IRAB: intent={intent_id} source={:?} focused_object={:?}",
        input.source,
        input.context.focused_ref()
    );
}

fn target_label(target: &saai_entity_store::ObjectRef) -> String {
    match target {
        saai_entity_store::ObjectRef::Entity { id } => short_id(*id),
        saai_entity_store::ObjectRef::Space { id } => id.clone(),
    }
}

#[cfg(test)]
mod confirmation_gate_tests {
    use super::Daemon;

    #[test]
    fn a_registered_read_only_action_does_not_require_confirmation() {
        // `display.inspect`'s own spec sets `requires_confirmation:
        // false` -- it is genuinely read-only (see its own
        // description). This is the one real gap the original
        // `process_resolved_action` had: nothing distinguished this
        // case from a future mutating one, so both would have
        // auto-completed identically.
        assert!(!Daemon::action_requires_confirmation("display.inspect"));
    }

    #[test]
    fn an_unregistered_action_id_fails_closed_into_requiring_confirmation() {
        // Never fail-open: an action id this lookup cannot vouch for
        // must be treated the same as one that explicitly requires
        // confirmation, not the same as one explicitly cleared for
        // auto-completion.
        assert!(Daemon::action_requires_confirmation(
            "storage.delete_everything"
        ));
    }
}

#[cfg(all(test, unix))]
mod follow_space_tests {
    use super::Daemon;
    use saai_entity_protocol::{ClientRequest, ResponseResult, ServerMessage};
    use std::io::{BufRead, BufReader as StdBufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream as StdUnixStream};
    use std::thread;

    fn respond(stream: &StdUnixStream, message: &ServerMessage) {
        let mut encoded = serde_json::to_vec(message).unwrap();
        encoded.push(b'\n');
        (&*stream).write_all(&encoded).unwrap();
    }

    fn read_request(reader: &mut impl BufRead) -> Option<ClientRequest> {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        serde_json::from_str(&line).ok()
    }

    fn serve_empty_lists(stream: StdUnixStream) {
        let mut reader = StdBufReader::new(stream.try_clone().unwrap());
        while let Some(request) = read_request(&mut reader) {
            match request {
                ClientRequest::Subscribe { request_id, .. } => respond(
                    &stream,
                    &ServerMessage::success(request_id, ResponseResult::Subscribed),
                ),
                ClientRequest::ListEntities {
                    request_id,
                    space_id,
                    ..
                } => respond(
                    &stream,
                    &ServerMessage::success(
                        request_id,
                        ResponseResult::Entities {
                            space_id,
                            entities: vec![],
                        },
                    ),
                ),
                other => panic!("unexpected request {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn follow_selected_space_retargets_the_watch() {
        let temp = tempfile::tempdir().unwrap();
        let socket = temp.path().join("entityd.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            serve_empty_lists(stream);
        });
        let mut daemon = Daemon::connect(&socket, "work".into(), "127.0.0.1:1".into())
            .await
            .unwrap();
        assert_eq!(daemon.watch_space(), "work");
        assert!(!daemon.follow_selected_space("work".into()).await.unwrap());
        assert!(daemon.follow_selected_space("home".into()).await.unwrap());
        assert_eq!(daemon.watch_space(), "home");
        drop(daemon);
        handle.join().unwrap();
    }
}
