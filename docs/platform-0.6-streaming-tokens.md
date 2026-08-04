# Platform 0.6 — token / chunk streaming

## What streams

When a client uses `diagnose` with `stream: true` (Stage / TUI):

1. Tool / policy progress frames (unchanged)
2. **`assistant_delta`** — text chunks while the model generates
3. Final **`assistant`** — full text (and tool calls afterward)

## Provider API

`ModelProvider::complete_with_progress(..., on_delta)`:

| Provider | Behavior |
|---|---|
| Mock | Splits assistant text into word chunks |
| OpenAI-compat | HTTP `stream: true` SSE (`data:` lines) |
| Fallback | Tries each provider’s streaming path |

Non-streaming `complete()` remains for callers that do not need deltas.

## UI

Stage appends deltas into a live “SaaiOS” beat with a caret; final `assistant` finalizes the same beat.
TUI ignores delta noise and shows the final assistant line.

## Docs

See also [Stage GUI](platform-0.6-gui.md).
