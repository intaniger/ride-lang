# ride

A small pure functional language over numbers.

A program declares a STATE record and a set of functions. Evaluating the entry function against
one STATE record yields **one number**. There is nothing else in the language: no strings, no
collections, no effects, and no state carried between calls.

```ride
state { s }

// Shared by more than one arm, so it is named once rather than written twice.
let phase x = 0.6133x - 220.788

let main =
    if s < 345 then 350.1
    else if s < 360 then 0.9267s + 30.3885
    else if s < 570 then s + 5 - cos(phase(s))
    else 1.2933s - 161.181
```

This is **not** an animation system and not a renderer. It computes numbers. A host that wants
to animate something calls it and does the animating itself.

## Documentation

**[`docs/`](docs/README.md) is the handbook** — nine parts that take a reader with no context to
the point of contributing safely. Start at [`docs/README.md`](docs/README.md).

| Part | For |
|------|-----|
| [1 Overview](docs/01-overview.md) | the map, the three lifecycle bands, the drift-risk table |
| [2 Language](docs/02-language.md) | grammar, syntax, precedence, the twelve builtins |
| [3 Compiler](docs/03-compiler.md) | the five stages, frame layout, the diagnostic contract |
| [4 Bytecode](docs/04-bytecode.md) | all 31 instructions and their stack effects |
| [5 Verification](docs/05-verification.md) | Proof A, Proof B, the twenty error codes |
| [6 Runtime](docs/06-runtime.md) | the dispatch loop, a traced frame, the four guarantees |
| [7 Testing](docs/07-testing.md) | the gate, the hostile suite, the differential harness |
| [8 Recipes](docs/08-recipes.md) | add an instruction, end to end |
| [9 Roadmap](docs/09-roadmap.md) | done, missing, and the order to build it in |

## Why it looks like this

Four properties are load-bearing. Each one buys a guarantee the runtime depends on.

| property | what it buys |
|----------|--------------|
| **first-order** — a function is never a value | every call is direct and saturated, so there is no closure to allocate; and implicit multiplication (`15exp(-x)`) is unambiguous, because application is always `f(a, b)` |
| **strict** — call by value | the reading order is the evaluation order |
| **no recursion** — the call graph is acyclic | the frame array can be sized at compile time |
| **total branches** — every `if` has an `else` | both arms leave the stack at the same height |

Breaking any of them invalidates a proof in `runtime/src/verify.rs`.

## Layout

```
src/                     the compiler — Langium front end, TypeScript
  ride.langium           the grammar; the header states the invariants
  compile.ts             five stages: collect, check, acyclic, emit, link
  evaluate.ts            a reference evaluator, for tests and differential checking
  index.ts               parse + compile; the only call a host needs
  generated/             Langium output — committed, never hand-edited
runtime/                 the runtime — Rust, no dependencies, compiles to WASM
  src/op.rs              the instruction set
  src/verify.rs          Proof A (operand stack) and Proof B (frames, call depth)
  src/eval.rs            the dispatch loop, on three fixed arrays
test/                    front-end tests
examples/                programs the differential harness sweeps
scripts/differential.mjs generates a Rust test from the TS evaluator's output
```

The compiler and the runtime are deliberately separate processes with one artifact between
them: bytecode. The runtime never parses, which is what lets it ship with no dependencies.

## The two proofs

Bytecode can reach the runtime without passing through the compiler, so nothing is evaluated
until both proofs pass. `Verified` cannot be built any other way, and the evaluator accepts
nothing else — it indexes its arrays with no run-time guard, because the proofs are the guard.

**Proof A — the operand stack, per function.** A single accumulating scan over the instruction
array is sound only for straight-line code: it assumes the next instruction always runs after
this one, and a jump breaks that. So Proof A keeps a height table and checks it *at each jump
target* rather than accumulating. Two jumps to one target must agree. This is the rule a
WebAssembly validator applies at a block boundary, and it is sound here only because jumps are
forward-only.

**Proof B — frames and call depth.** Proof A bounds one function; it says nothing about how many
frames are live at once. The language forbids recursion, so the call graph is acyclic — and
`verify` is where that stops being a convention and becomes a checked fact. With no cycle the
graph is a DAG, so the frame bound is the heaviest root-to-leaf path and the call-depth bound is
the longest one.

## Getting started

```bash
npm install
npm run langium:generate     # after any edit to src/ride.langium
npm run check                # both test suites — the gate
```

| command | what it does |
|---------|--------------|
| `npm test` | front-end tests (vitest) |
| `npm run test:runtime` | runtime tests (cargo) — needs no network and no npm |
| `npm run check` | both; the gate the lifecycle depends on |
| `npm run langium:generate` | regenerate the parser and AST from the grammar |
| `npm run build` | generate, then compile TypeScript |
| `npm run build:wasm` | compile the runtime to wasm — **builds, but exports nothing yet; see Status** |

### The differential harness

There are two implementations of one instruction set, and two implementations of anything drift.
`scripts/differential.mjs` compiles every example with the front end, records what the
TypeScript evaluator returns, and writes those samples into a Rust test as literals.

```bash
npm run build && node scripts/differential.mjs && npm run test:runtime
```

It uses a relative tolerance rather than exact equality, for one stated reason: the arithmetic,
comparison, branch and call instructions are bit-exact across both evaluators, but the
transcendentals are not. `Math.cos` computes in `f64` and is rounded to `f32`, while Rust's
`f32::cos` computes in `f32` throughout, so the two can differ in the last bit. The tolerance
absorbs that and nothing larger — a wrong branch or a wrong frame slot moves the result by
orders of magnitude.

## One decision is open

**The language has one number type.** Adding a separate `int` is a one-way door, and it is not
settled: `f32` division by zero yields an infinity, but `i32.div_s` *traps*, so `int` would
introduce a trap path into code that must not panic. `CLAUDE.md` §10 states the three available
answers. Until one is chosen, the language stays single-type.

## Status

A scaffold, and green: 69 front-end tests, 35 runtime tests, 2 differential tests, 1 doctest.
The grammar, the five compiler stages, both proofs, the evaluator and the differential harness
are in place.

**The one real gap is the seam between the two halves.** The compiler emits a TypeScript
object; the runtime consumes a Rust struct; nothing serialises between them. Today the only
bridge is `scripts/differential.mjs`, which writes Rust *source literals* — fine as a test, not
a delivery mechanism. So `npm run build:wasm` compiles (362 bytes) but exports nothing, because
the crate carries no `#[wasm_bindgen]` surface yet.

Three things follow from that, in order:

1. **A bytecode wire format**, with a writer in the compiler and a reader in the runtime. The
   reader is untrusted input, so it runs both proofs before anything is evaluated — that is
   already what `Verified::new` requires.
2. **The `#[wasm_bindgen]` surface**: load bytes, run both proofs, evaluate against a STATE
   record written into linear memory once per call.
3. **A CLI** to compile a `.ride` file to that format.

Not started, and not needed to evaluate the design: no language server, no editor extension,
no `import` resolution across files (the grammar parses `import`, the compiler does not yet
follow it).
