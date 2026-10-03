# Board

The single source of truth for scheduling state. Plan documents in `.claude/tasks/` hold prose
only — never a status. See `CLAUDE.md` §4 for the state machine and §3.4 for the write-set.

Surface ids are defined in `CLAUDE.md` §10 under **Surfaces**.

| plan | status | priority | phase | deps | writes | debt `D` |
|------|--------|----------|-------|------|--------|----------|
| _(none yet)_ | | | | | | |

### Known gap, no plan written yet

The compiler emits a TypeScript object and the runtime consumes a Rust struct. **Nothing
serialises between them.** `scripts/differential.mjs` bridges the two by writing Rust source
literals, which works as a test and not as a delivery path. So `npm run build:wasm` produces a
module with no exports.

The next plan should cover, in this order: a bytecode wire format (writer in `C`, reader in a
new runtime surface); the `#[wasm_bindgen]` entry points; then a CLI. The reader takes untrusted
input, so it must run both proofs before evaluating — which `Verified::new` already enforces.

Also unimplemented: `import` parses in the grammar (`G`) but the compiler (`C`) does not follow
it across files.

## Status values

`proposed` · `answered` · `accepted` · `in-progress:N` · `verified:N` · `implemented` ·
`in-review` · `approved`

`accepted` is the only state `begin` is defined from. `approved` is terminal, and shipping
happens after it.

## Notes

- The gate is `npm run check`. It must be green at every phase boundary (`CLAUDE.md` §9: the
  test gate is never deferrable).
- A change to the instruction set, the proofs or the evaluator also owes a regenerated
  differential test — see §10 **Review procedure**.
