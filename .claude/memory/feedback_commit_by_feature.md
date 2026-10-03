---
name: feedback-commit-by-feature
description: Commit by feature slice, each commit a working end-to-end step. Never by layer or by file.
metadata:
  type: feedback
---

Cut commits by **feature**, not by **layer**. One commit is one small capability that goes
through every layer it needs: grammar, generated parser, compiler, evaluator, test. After the
commit, a user can do something they could not do before, and the tests are green.

Do not write one commit for "the grammar", one for "the runtime", and one for "the compiler".

**Why:** the owner said of `b23f8f4`: "did a whole grammar into a single file / commit which is
very 'vertical' progress: by meaning of adding grammar, compiler, and so on. But not an actual
development progress." And: "If I do it by hands I will start with minimal grammar first, then
compiler, evaluator, and so on. Even runtime `runtime/` might not be included here." And: "So I
want to divide commit again by feature (horizontal) not by file (vertical)."

**How to apply:**

- Before the first commit of multi-layer work, list the feature ladder. Start with the smallest
  program that goes all the way through (`let main = 2 + 3 * 4`). Add one capability per rung.
- Each rung edits the grammar, the generated parser, the compiler, the evaluator and the tests
  together. If the grammar changes, run `npm run langium:generate` in the same commit.
- Add a field, an instruction or a diagnostic in the rung that first reads it. Do not declare it
  early. One exception: a data layout that hand-built test fixtures construct (`Func` in the
  runtime tests) may arrive whole, so a later field does not touch every fixture.
- A comment must not name a file that does not exist yet. Write the comment for the commit it
  is in, and restore the fuller text in the commit that adds the file.
- The runtime comes after the TypeScript front end, as its own short ladder. The owner hinted
  it may not belong in the early commits at all.
- Gate every commit, not only the last. Check out each commit in a clean tree and run
  `tsc --noEmit`, `npm test`, `cargo test`, and a `langium generate` that leaves
  `src/generated` unchanged.
- When rewriting history, the final tree must equal the old tip byte for byte. Check it with
  `git diff <old-tip> <new-tip>`. Tag the old tip first.

Related: [[project-node-22-required]]
