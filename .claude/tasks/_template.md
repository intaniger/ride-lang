# <plan name>

> Copy this file to `.claude/tasks/<slug>.md`. Prose only — the status lives on the board
> (`_index.md`), never here. Write every section in ASD-STE100 (`CLAUDE.md` §5): short
> single-clause sentences, active voice, one meaning per word. Identifiers, paths and
> `file:line` citations stay exact.

## Summary

<One paragraph. What changes, and what the reader will be able to do afterwards.>

## Context

<Why now. What is true today that makes this worth doing. Cite the code: `file:line`.>

## Decision

<What will be built. Name the alternative that was not chosen, and say what it would have cost.>

## Architecture

<An ASCII diagram before any numbers, when the change has a shape. Then the mechanism.>

## Non-goals

<What this plan does NOT do, so a reviewer does not ask for it.>

## Open questions

<Decisions owed to the owner. Each one a question with its options and their costs — never a
placeholder. The plan cannot reach `accepted` while any remain (`CLAUDE.md` §4.3).>

## Constraints

<Which of the §10 architectural commitments this change touches, and how it respects each one.
If it trades one away, say so here and let the owner decide.>

## Phases

<One phase per row. Each must be small enough to trace line by line, and large enough to leave
a testable result. The gate for every phase is `npm run check`.>

| # | what | writes | gate |
|---|------|--------|------|
| 1 | | | `npm run check` |

## References

<The source for each load-bearing claim. A tier-1 reference, or a verified `file:line`.>
