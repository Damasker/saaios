# Platform 0.5 — auto-diagnose & event feed

## Auto-diagnose

When enabled, high-CPU telemetry can run a real diagnose without a user typing
the question.

Config:

```toml
[automation]
enabled = true
auto_diagnose = true
```

CLI: `--auto-diagnose` / `--no-auto-diagnose`

This enables rule `high-cpu-auto-diagnose` (`AutomationAction::AutoDiagnose`)
and starts a runtime worker that:

1. Listens for `AutomationAutoDiagnose`
2. Calls `AiRuntime::handle_user_text(prompt)`
3. Emits `AutomationDiagnoseCompleted` or `AutomationDiagnoseFailed`

Cooldown: `SuggestDiagnose` remains a soft suggestion; `AutoDiagnose` executes.

Cooldownoldown for auto-diagnose rule: 120s (stricter than suggest/notify).

## Event feed

Runtime keeps a ring buffer of recent `Automation*` bus events.

UDS: `EventsTail { limit }`  
TUI: press `e`

Status includes `auto_diagnose: bool`.
