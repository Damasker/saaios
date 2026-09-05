# Platform 0.5 — multi-turn chat + progress streaming

## Multi-turn sessions

Chat history is process-local and orthogonal to **policy session grants**.

| Field | Meaning |
|---|---|
| `session_id` | UUID for conversation transcript (auto-created on first diagnose) |
| Policy `session` grant | Tool allow-list for the runtime process lifetime |

### Protocol operations

```json
{"op":"diagnose","text":"Почему тормозит?","session_id":null,"stream":false}
{"op":"diagnose","text":"Какой pid?","session_id":"<uuid>","stream":true}
{"op":"chat_reset","session_id":"<uuid>"}
{"op":"confirm","correlation_id":"...","call_id":"...","tool":"...","arguments":{},"scope":"once","session_id":"<uuid>"}
```

Responses include `session_id`. Confirm with `session_id` appends the tool result into that conversation so the next turn can refer to it.

History keeps the last ~40 non-system messages (user / assistant / tool_result turns). System prompt + memory facts are rebuilt each turn.

## Progress streaming

`diagnose` with `"stream": true` keeps the connection open and writes **NDJSON**:

1. `{"type":"progress","event":{...RuntimeEvent...}}` — tool_call / tool_result / policy / assistant / confirmation / completed
2. `{"type":"done", ...ClientResponse fields...}` — final summary, pending confirm, grants

Non-stream diagnose still returns a single JSON object and may include a `progress` array of the same events.

## Console

- Sessions persist across Enter presses
- `/new` (or `/reset`) clears the chat session
- Diagnose uses `stream: true` and shows tool progress lines while waiting
- The default transport is the Unix socket from `--sock` / `SAAIOS_SOCK`.
- `--tcp HOST:PORT` / `SAAIOS_TCP` connects the same console to a TCP runtime.
  Pixel 7 uses the USB-only endpoint `172.31.7.1:38127`.
- A runtime started with `--tcp` keeps its Unix socket active in parallel, so
  on-device clients do not depend on USB interface routing.

## Non-goals (this slice)

- Token-level LLM SSE (OpenAI `stream: true`) — deferred; progress frames cover tool/assistant steps
- Shared-memory data plane (ADR-002)
- Durable chat transcripts across runtime restarts
