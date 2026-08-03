# Platform 0.2 — automation and budgets

## Event-driven automation

`automation-engine` watches the event bus and fires rules.

Default rules:

- `system.metrics` tool result with `cpu_usage >= 85` → notify + suggest diagnose
- named event `TemperatureWarning` → notify

Actions are **non-executing** by default: they emit bus/audit events
(`AutomationNotify`, `AutomationSuggestDiagnose`). Dangerous tools still require
policy + user confirmation via the normal AI path.

Disable with:

```bash
saaios-runtime --no-automation
```

## Resource budgets

AI runtime enforces:

| Budget | Default | Flag / env |
|---|---|---|
| max concurrent requests | 1 | `--max-concurrent` / `SAAIOS_MAX_CONCURRENT` |
| request timeout | 30s | `--request-timeout-secs` / `SAAIOS_REQUEST_TIMEOUT_SECS` |
| max tool iterations | 6 | (code default) |

A second request while the slot is busy fails fast with `AI runtime busy`.
