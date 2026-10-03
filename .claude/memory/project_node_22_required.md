---
name: project-node-22-required
description: The front end needs Node 22 or later. Node 18 and 20 fail on `Object.groupBy` in vitest and in `langium generate`. `.nvmrc` pins 22.
metadata:
  type: project
---

The front end needs Node 22 or later. chevrotain 12.0.0, which langium 4.3 uses, declares
`"node": ">=22.0.0"` and calls `Object.groupBy`. On Node 18 or 20 two things fail with
`TypeError: Object.groupBy is not a function`:

- `npm run langium:generate`. It writes nothing.
- every vitest test that parses, because the parser is built when the first test runs.

Node 21 also runs, but vitest 3.2.7 does not support it (`^18 || ^20 || >=22`). Use 22.

The repository pins it, since 2026-10-04:

- `.nvmrc` holds `22`. mise reads it. On the owner's machine `mise activate zsh` runs after
  nvm in `~/.zshrc`, so a shell in the repository gets mise's Node 22.18.0. Outside it, nvm's
  default is v18.17.1.
- `package.json` `engines.node` is `>=22.0.0`.
- `.vscode/settings.json` sets `vitest.nodeExecutable` to `~/.local/share/mise/shims/node`. The
  Vitest extension spawns `node` from VS Code's own PATH, which is nvm's 18 and ignores
  `.nvmrc`. The shim runs in the repository root, so it reads `.nvmrc`.

**Why:** a failed generate leaves `src/generated/` unchanged. The tests then pass against a
grammar that is no longer the source of truth, which is the failure `CLAUDE.md` §10 warns about.
On 2026-10-04 the owner hit the same error in the VS Code test panel while `bun test` passed in
the terminal. Bun's runtime has `Object.groupBy`, but `bun test` is Bun's runner, not the vitest
gate in `npm run check`.

**How to apply:** run `node --version` in the repository before a grammar edit or a test run.
If it is below 22, the mise hook did not run. Use `~/.local/share/mise/shims/node` from the
repository root. After `langium:generate`, run `git status --porcelain src/generated` and read
it. If the VS Code test panel shows `Object.groupBy`, check `vitest.nodeExecutable` and reload
the window.

Related: [[feedback-commit-by-feature]]
