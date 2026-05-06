# Support

Use the right path so questions and bugs do not get mixed together.

## Before Opening Anything

Check:

- [`README.md`](./README.md)
- [`docs/README.md`](./docs/README.md)
- [`devops/README.md`](./devops/README.md)
- [`docs/architecture/project-brief.md`](./docs/architecture/project-brief.md)

For workstation problems, run:

```powershell
just vs-status
just doctor
```

## Use These Paths

### Bug reports

Open a GitHub issue with the bug template when:

- the compiler miscompiles or crashes
- a documented command fails unexpectedly
- CI, SDK packaging, or Python integration behavior regresses

Include the exact command, host environment, expected behavior, and actual behavior.

### Feature requests

Open a GitHub issue with the feature template when:

- you are proposing a language feature
- you want a new workflow, tool, or platform capability
- you need a packaging, SDK, or integration enhancement

### Performance regressions

Open a GitHub issue with the performance template when:

- a benchmark gets slower
- compile time, binary size, or memory usage regresses measurably
- a backend or optimization change hurts a concrete workload

### Security issues

Do not file public issues. Follow [`SECURITY.md`](./SECURITY.md).

## Support Boundaries

This repo is a compiler project, not a general consulting channel.

Good support requests are:

- reproducible
- scoped
- benchmark-backed when the claim is about performance
- tied to a concrete command, file, or workflow
- grounded in the current repo rather than speculative future features

## No SLA Promise

Maintainer time is finite. High-signal, reproducible reports get attention first.
