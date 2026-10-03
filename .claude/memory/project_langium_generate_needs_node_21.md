---
name: project-langium-generate-needs-node-21
description: `npm run langium:generate` fails on Node 18 and 20 with `Object.groupBy is not a function`. Use Node 21 or later.
metadata:
  type: project
---

`npm run langium:generate` needs Node 21 or later. On Node 18 it stops with
`TypeError: Object.groupBy is not a function` and writes nothing. `package.json` says
`"node": ">=20.10.0"`, which is too loose: Node 20 also lacks `Object.groupBy`.

Observed on 2026-10-03 on the owner's machine. The default Node was v18.17.1. The nvm install
`v21.7.1` works. With it the generated parser reproduces byte for byte, so `src/generated/`
is deterministic.

**Why:** a failed generate leaves `src/generated/` unchanged. The tests then pass against a
grammar that is no longer the source of truth, which is the failure `CLAUDE.md` §10 warns about.
It looks like a clean run unless the exit code is read.

**How to apply:** before any grammar edit, run `node --version`. If it is below 21, put a newer
one first on `PATH` for that shell, for example
`PATH="$HOME/.nvm/versions/node/v21.7.1/bin:$PATH"`. After `langium:generate`, run
`git status --porcelain src/generated` and read it. Offer to raise `engines.node` in
`package.json` to `>=21`. Do not change it without asking.

Related: [[feedback-commit-by-feature]]
