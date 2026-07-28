# Security Policy

Agam is still pre-1.0, but security issues should be handled with production discipline.

## Supported Security Fix Window

Security fixes are expected to land on:

- the current `main` branch
- the newest supported release artifacts once formal release channels are in active use

Older snapshots, local forks, and stale generated SDK bundles should be treated as unsupported unless
explicitly called out by maintainers.

## How To Report a Vulnerability

Do not open a public issue with exploit details.

Preferred path:

1. Use GitHub private vulnerability reporting for the repository if it is enabled.
2. If that is unavailable, open a minimal support request without sensitive details and ask for a
   private handoff path.

Include:

- affected component or path
- impact and attacker assumptions
- reproduction steps
- proof-of-concept details if needed
- suggested mitigation if you have one

## Response Expectations

Maintainers should aim to:

- acknowledge the report quickly
- reproduce and classify severity
- land a fix or mitigation on `main`
- publish advisory notes once the issue is safe to disclose

## Scope

Security-relevant areas in this repo include:

- `agamc exec` and headless execution boundaries
- runtime sandboxing and policy enforcement
- SDK packaging and artifact validation
- registry and package publication flows
- Python wrapper execution bridges
- CI/release automation

## What Not To Use This For

Use normal issues for:

- feature requests
- compiler crashes without security impact
- portability bugs
- support questions
