# AGENTS.md

Session-bootstrapping instructions for agents working in this repo. Code-style
and dependency-strategy rules live in `docs/CODING.md`, not here.

## Superpowers

This repo assumes the [superpowers](https://github.com/obra/superpowers)
plugin/skill system is available. If it is not installed, alert the user
before proceeding.

## Read the top-level docs first

At the start of every session, read the following files in full:

- `docs/ARCHITECTURE.md`
- `docs/BACKLOG.md`
- `docs/CODING.md`
- `docs/DEV.md`
- `docs/VISION.md`

These are the only files directly under `docs/`. Everything else lives in a
subdirectory (`docs/decisions/`, `docs/porting/`, `docs/authoring/`,
`docs/superpowers/`) and is out of scope for this bootstrap step - read those
on demand when the task at hand references them.
