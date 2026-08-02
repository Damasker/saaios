# ADR-001: SaaiOS Platform does not depend on a custom kernel

## Status

Accepted

## Context

Building a custom kernel and an AI-native product in one critical path risks turning SaaiOS into a bare-metal research project with no usable AI platform.

## Decision

- SaaiOS Platform is implemented first on Linux.
- All host dependencies are hidden behind a Platform/API adapter boundary.
- A native kernel may later become an alternate backend.
- Userspace components must not assume Linux-only APIs at the business-logic layer.

## Consequences

- Product hypotheses can be tested immediately.
- Kernel work is isolated in a separate repository/track.
- Some Linux adapters (`/proc`, etc.) are acceptable early, as long as tools talk to traits/interfaces.
