# ride — Handbook

**Start here.** This set of documents takes a reader with no prior context and ends with enough
detail to change the language safely.

Read the parts in order. Part 1 gives the map. Parts 2 to 6 follow one source file through the
system, in the order the code runs. Parts 7 to 9 are tools: the test harnesses, one end-to-end
recipe, and the roadmap.

---

## The parts

| Part | Document | Scope |
|------|----------|-------|
| 1 | [Overview](01-overview.md) | What `ride` is. The three lifecycle bands, and the one seam between the two halves. |
| 2 | [The language](02-language.md) | Grammar and surface syntax. Every form, and the four invariants that shape them. |
| 3 | [The compiler](03-compiler.md) | **Compile band** — everything that happens once per source file. Five stages. |
| 4 | [The bytecode](04-bytecode.md) | Reference. All 31 instructions, each with its stack effect and its two obligations. |
| 5 | [Verification](05-verification.md) | **Verify band** — the trust boundary. Proof A and Proof B, in full. |
| 6 | [The runtime](06-runtime.md) | **Evaluate band** — the dispatch loop. One pass, three fixed arrays, no allocation. |
| 7 | [Testing](07-testing.md) | The gate, the hostile-program suite, and the differential harness. |
| 8 | [Recipes](08-recipes.md) | Add an instruction. The one change that touches every layer. |
| 9 | [Roadmap](09-roadmap.md) | What is done. What is missing. What a full version needs. |

---

## In one paragraph

`ride` is a small pure functional language over numbers. A program declares a STATE record and a
set of functions. Evaluation of the entry function against one STATE record gives **one number**.
The language has no strings, no collections, no effects, and no state between calls.

The project has two halves. A Langium front end in TypeScript compiles source to bytecode. A Rust
runtime verifies that bytecode and evaluates it. The runtime has no dependencies and allocates
nothing per call.

---

## What to read for a given task

| Task | Read |
|------|------|
| Understand the project | Part 1, then Part 2 |
| Add a builtin function | Part 4, then Part 8 |
| Add an instruction | Part 8. It names every file in order. |
| Change the grammar | Part 2, then Part 3 §03 |
| Understand why a program was rejected | Part 5 §05 — the error table |
| Find out what is not built yet | Part 9 |
| Run the tests | Part 7 §01 |

---

## Three facts to carry into every part

1. **The runtime never parses.** The front end is the only parser. This is what keeps the runtime
   free of dependencies.

2. **Bytecode can arrive without passing through the compiler.** So the runtime re-proves every
   safety property itself, in Part 5. The compiler's checks are for the author. The runtime's
   proofs are for the machine.

3. **The project is polyglot, and some values live in two places.** Part 1 §05 lists every
   duplicate and what breaks when the copies drift. Read that table before any change.

---

> Sources read for this handbook: `src/ride.langium`, `src/compile.ts`, `src/evaluate.ts`,
> `src/index.ts`, `runtime/src/op.rs`, `runtime/src/verify.rs`, `runtime/src/eval.rs`,
> `runtime/tests/runtime.rs`, `test/compile.test.ts`, `scripts/differential.mjs`,
> `package.json`, `runtime/Cargo.toml`, and `CLAUDE.md`. Line references point at the source as
> read on **2026-10-03**. **Names and rules are stable. Line numbers move.**
