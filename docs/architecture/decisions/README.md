# Architecture Decision Records

This directory holds architecture decision records (ADRs) for cross-cutting technical choices that
should outlive an issue thread or pull request.

## When To Add One

Add an ADR when a change sets or changes a durable contract for:

- compiler architecture or lowering boundaries
- runtime or execution policy
- packaging, SDK, or integration shape
- repository layout or tooling workflow
- benchmark methodology or performance policy

## What An ADR Should Capture

Keep it short and concrete:

1. context
2. decision
3. consequences
4. status

Use [`0000-template.md`](./0000-template.md) as the starting point.

## Naming

Use a zero-padded numeric prefix and a short slug:

- `0001-devops-canonical-automation-surface.md`
- `0002-native-windows-primary-host-lane.md`

Do not rewrite history casually. If a decision changes, supersede the old ADR with a new one and
link both files.
