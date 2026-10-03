# Session log — 2026-10-03 — rewrite the history by feature

## 1. Initial Prompt

```
# One step at a time 

Commit b23f8f4d536f5d6fcb1565453efffee9af4baaf6 did a whole grammar into a single file / commit which is very "vertical" progress: by meaning of adding grammar, compiler, and so on. But not an actual development progress.

If I do it by hands I will start with minimal grammar first, then compiler, evaluator, and so on. Even runtime `runtime/` might not be included here. 

So I want to divide commit again by feature (horizontal) not by file (vertical)
```

## 2. What The Agent Understood

The five commits from `b23f8f4` to `920e7a4` each add one layer: grammar, runtime, compiler,
harness, docs. The owner wants the same final code, cut in the order a person would write it.
Each commit is one small feature that goes through every layer it needs. The scaffold
(`eb937f5`) and `CLAUDE.md` (`03dcabe`) stay as they are. The runtime may come late.

## 3. What The Agent Did

1. Read the history, the grammar, the compiler, the evaluator, the runtime and both test files.
2. Found that `origin/main` equals local `main`, so a rewrite needs a force-push. Left `main`
   and `origin` alone.
3. Tagged the old tip `vertical-history`. Built a clean worktree on branch `by-feature` from
   `03dcabe`, so the ignored `runtime/tests/differential.rs` could not leak into early commits.
4. Found that `langium generate` fails on Node 18. Used Node 21.7.1 from nvm on `PATH`.
5. Built 17 commits, one feature each: constants, state, power and unary minus, implicit
   multiplication, builtins, comparison and `if`, `let … in`, functions, no recursion, `import`
   (parse only), a worked program; then the runtime in three (straight-line, branches, locals
   and calls); a grammar-comment commit; and the two original single-feature commits
   cherry-picked unchanged (differential harness, docs).
6. Derived most intermediate files from the final files by cutting out later features. Used
   `git checkout 920e7a4 -- <file>` at the last commit that touches a file.
7. Checked every commit in a clean worktree: `langium generate` leaves `src/generated`
   unchanged, `tsc --noEmit`, `npm test`, `cargo test`, and `npm run check` from the first
   runtime commit on. Ran the differential harness at the tip.
8. Confirmed `git diff 920e7a4 by-feature` is empty at `52b5ad6`.

Two of the agent's own messages were wrong and were amended before the sweep: the claim that
`2x^2` is `2(x^2)` (it is `(2x)^2`, verified by running it), and a miscount of rejecting tests.

## 4. What The Owner Changed Manually

None so far. The owner has not yet reviewed the branch.

## 5. The Correct Understanding

Not confirmed by the owner yet. The working reading is the one in §2. Two choices were the
agent's and are open: the TypeScript front end comes before the whole runtime, and the docs stay
one commit at the end.

## 6. Miscommunication

None known. The prompt gave no ladder, so the order of features is the agent's proposal.

## 7. Next Steps

1. Review `by-feature` and the commit list, then say whether to keep this order.
2. Decide whether to reset `main` to `by-feature` and force-push with `--force-with-lease`
   to `origin/main`. This replaces published history, so it needs the owner's explicit word.
3. Raise `engines.node` in `package.json` to `>=21`, if the owner agrees.
4. Fix the `let` slot bug in `src/compile.ts` (`isLetIn` branch) in a separate plan. A
   shadowing `let` reuses its name's slot and never restores it, so
   `let a = 1 in (let a = 2 in a) + a` gives 4, not 3. The rewrite kept it on purpose.
5. Decide what the compiler does with `import` in `src/compile.ts`. It ignores it today.

## 8. What Worked

None yet. The owner has not accepted or corrected anything.
