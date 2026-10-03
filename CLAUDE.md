# CLAUDE.md — Software Development Method (portable)

This file is a **method**, not a project. It tells an AI agent how to plan work, how to move a plan
through states, how to write prose, and how to keep project knowledge current.

It covers four subjects:

1. **The plan document (RFC)** — how to write one, and when one is required.
2. **The lifecycle** — the states a plan moves through, and who fires each transition.
3. **Plain language (ASD-STE100)** — the prose standard for every plan document.
4. **Knowledge keeping** — memory, session logs, and the documents that hold project context.

It does **not** cover code review. The lifecycle keeps a `in-review` state, but the review procedure
is out of scope. Supply your own, or make the state a simple owner check.

**To adopt:** copy this file into a project as `CLAUDE.md`. Then add a project section at the end
(§10). Do not delete §1–§9. Do not change the state machine.


> **This copy is composed from two sources.** §1–§9 below are that method, unchanged. §10 is the
> project section it asks for. §11 is a behavioural layer adopted from Andrej Karpathy's
> `CLAUDE.md`, which covers how to write a single edit — the one subject the method leaves open.
> On conflict, §1–§10 win.
---

## 1. Adopt — the files this method expects

Create these before the first plan. The method reads and writes them.

| Path | Required | Purpose |
|------|----------|---------|
| `.claude/tasks/` | yes | One markdown file per plan (RFC). Prose only. |
| `.claude/tasks/_index.md` | yes | The **board**. The single source of truth for scheduling state. |
| `.claude/tasks/_template.md` | yes | The section skeleton every new plan copies. |
| `<memory>/MEMORY.md` | yes | The memory index. One line per memory file. Loaded each session. |
| `<memory>/session_log_*.md` | yes | One log per working session. |
| `<memory>/feedback_*.md` | yes | One durable rule per file, extracted from owner feedback. |
| `.claude/FOUNDATIONS.md` | optional | Domain knowledge cards. Add it when the project has a hard domain. |

`<memory>` is whatever directory the agent harness persists. Keep every memory file in one directory.

**One more thing is required: an automated test command.** Name the exact command in §10 — the test
command, the lint command, the build command. The lifecycle depends on it. A project with no
runnable test command cannot use the `verified` state honestly.

The word **gate** has exactly one meaning in this file: a condition a phase must pass to exit. There
are two of them, and no others — the **test gate** and the **comprehension gate** (§3.3).

---

## 2. Roles

Two roles exist. Keep them separate.

- **The owner** — the human. The owner decides. The owner holds the pen on scope, on design forks,
  and on every state transition.
- **The agent** — the AI. The agent researches, writes plans, builds, verifies, and keeps documents
  current.

### The asymmetry

The agent **never** does these things:

- Fire `approve` (T3) or `begin` (T4). Approval is the owner's word, not an inference.
- Defer a comprehension question on its own initiative.
- Open review (T8) unasked.
- Resolve an Open Question by picking the option it prefers.

The agent **is encouraged** to do these things without asking:

- Edit a plan document that went stale. Fold in a resolved answer. Correct a wrong `file:line`.
- Consolidate a plan document that grew a stack of update banners.
- Write a session log and a feedback file at the end of a session.
- Report a defect it finds in its own earlier work.

**The rule in one line: documents are the agent's to keep current. State transitions are the
owner's.** After a document edit, show the diff. The owner must be able to trace it.

---

## 3. The plan document (RFC)

### 3.1 When a plan is required

Write a plan before you implement a **feature**, a **behaviour change**, or a **refactor that moves
code across a module boundary**.

Do not write a plan for a typo fix, a comment, a rename inside one file, or a mechanical edit the
owner asked for directly.

An order to build is **not** permission to build without a plan. If the owner says "build it now"
and no plan exists, write the plan first. Then build.

### 3.2 The sections

Copy `_template.md`. Every plan has these sections, in this order.

| Section | Contains | Test of a good one |
|---------|----------|--------------------|
| **Summary** | The problem, the decision, the accepted cost. One short paragraph. | A reader can decide whether to read further. |
| **Context** | What exists today. Why it is not enough. File paths. An ASCII diagram for any spatial or data-flow claim. | A reader who never saw the code can follow it. |
| **Decision** | The chosen approach, in 2–4 sentences. The verdict, not the argument. | A reader who wants only "what did we decide" can stop here. |
| **Architecture** | The options considered. The chosen one, marked. The reason each other option lost. | Every rejected option states why it lost. |
| **Implementation Plan** | Phases, in dependency order. Each phase declares its two gates. | See §3.3. |
| **Requirements** | Functional and non-functional checkboxes. | Each item is observable. |
| **Non-goals** | What this plan could do but will not do, and why. | Scope creep has a written wall. |
| **Open Questions** | Unresolved forks. Each one framed as a decision the owner owes. Write *None* when empty. | The owner can answer each one without reading the code. |
| **Constraints** | Hard rules this plan must not break. Cite the file each rule comes from. | Each constraint has a source. |
| **References** | The sources this plan stands on. See §6. | Each load-bearing domain claim has one. |

**Build `_template.md` from this table.** Write one heading per row, in this order, with the Contains
column as an HTML comment under each heading. A new plan then starts with the obligations already in
place.

The plan holds **prose only**. Status, priority, phase number, dependencies, and the write-set live
on the board (`_index.md`). Never put scheduling state in the plan document. Two homes for one fact
produce two different facts.

### 3.3 Phase design

A phase is the unit of build and the unit of trust.

**Size rule: one phase, one MVP.** Make each phase small enough that the owner can trace it line by
line. Make it large enough to produce a testable result. If a phase cannot be tested, it is not a
phase — it is half of one.

Each phase declares two gates:

1. **The test gate (objective).** A machine-checkable pass condition. Name the exact command and the
   exact test name. "It works" is not a test gate.
2. **The comprehension gate (subjective).** At the phase boundary the agent asks the owner **one
   unpredictable question, three lines or shorter**. The owner must answer it correctly.

Write both gates into the phase text **before** you build the phase.

**The comprehension question tests tracing, not calculus.** Ask what a specific line does. Ask what
breaks if a value changes. Ask which branch runs for a given input. Do not ask the owner to compute
a derivative. Do not ask a yes/no question — the owner can pass a yes/no question without reading
anything.

### 3.4 The write-set and collisions

Every plan declares, on its board row, the **set of files or surfaces it rewrites**. This is its
write-set.

Two plans that hold the same surface cannot run at the same time. Record the collision on the board
before the second plan starts, not after two edits conflict.

Name your surfaces once, in a legend at the top of the board. Keep the set small and closed. A
surface is a group of files that change together, not one file.

### 3.5 Well-formedness — four conditions

A plan is **not well-formed** until all four hold. A plan that is not well-formed cannot be
approved.

1. **Every load-bearing domain claim carries a reference.** See §6. A claim with no source becomes
   an Open Question, not an assertion.
2. **The prose meets §5** (ASD-STE100).
3. **Every phase declares both gates**, with the test gate named exactly.
4. **Open Questions is complete.** Every fork the agent noticed is listed. A fork the agent silently
   chose is a defect in the plan, not a decision.

---

## 4. The lifecycle

The plan moves through a state machine. The board's **Status** column holds the current state.

### 4.1 States

Eight states. Six flat, two indexed by phase number `N`, where `K` is the phase count.

```
proposed → answered → accepted → in-progress:N → verified:N → implemented → in-review → approved
```

- `proposed` — written, not approved. Open Questions may be open.
- `answered` — the owner answered the Open Questions. The agent has not folded them in yet.
- `accepted` — approved to build. **This is the end of planning.**
- `in-progress:N` — phase `N` is being built.
- `verified:N` — phase `N` passed both gates.
- `implemented` — every phase is verified.
- `in-review` — under review. **The review procedure is out of scope for this file.**
- `approved` — terminal.

### 4.2 The transitions

```
δ : (state, event) → state          A pair that is not listed is a REFUSAL, not a no-op.

T1   (proposed,      answer)                = answered
T2   (answered,      fold)                  = proposed
T3   (proposed,      approve) [OQ = ∅]      = accepted          ── ends PLAN
T4   (accepted,      begin)                 = in-progress:1
T5   (in-progress:N, pass)                  = verified:N
T5'  (in-progress:N, fail)                  = in-progress:N     (self-loop)
T6   (verified:N,    next)    [N < K]       = in-progress:(N+1)
T7   (verified:K,    finish)                = implemented
T8   (implemented,   open)                  = in-review
T9   (in-review,     approve) [review ✓ ∧ D = ∅] = approved     ── TERMINAL
T10  (in-progress:N | verified:N, replan)   = proposed          (re-plan in place)

guards
  OQ = ∅    No Open Question is open.
  pass      test ✓ AND comp ✓
  fail      NOT test ✓ OR NOT comp ✓
  test ✓    The test gate is green. NEVER deferrable.
  comp ✓    The owner answered the ≤3-line question — OR the owner explicitly deferred it,
            which books the question into the debt D.
  D = ∅     No deferred comprehension question is unanswered.
```

```
   ┌──────────┐   T1  owner answers      ┌──────────┐
   │ proposed │ ───────────────────────► │ answered │      PLAN
   │          │ ◄─────────────────────── │          │
   └──────────┘   T2  agent folds in     └──────────┘
        │ ▲
        │ └── T10  re-plan in place, from in-progress:N or verified:N
        │
        │ T3  owner approves   [guard: OQ = ∅]
        ▼
   ┌──────────┐
   │ accepted │   end of PLAN
   └──────────┘
        │ T4  begin  —  REFUSE this event from every other state
        ▼
   ┌───────────────┐ ◄── T5'  self-loop: test red, or wrong answer
   │ in-progress:N │ ◄── T6   N < K, continue at phase N+1        IMPLEMENT
   └───────────────┘
        │ T5  test ✓ AND comp ✓
        ▼
   ┌────────────┐ ── T6  ──► in-progress:(N+1)   when N < K
   │ verified:N │ ── T10 ──► proposed            on a design flaw
   └────────────┘
        │ T7  N = K
        ▼
   ┌─────────────┐
   │ implemented │
   └─────────────┘
        │ T8  owner asks for review
        ▼
   ┌───────────┐   REVIEW — the procedure is out of scope
   │ in-review │
   └───────────┘
        │ T9  review ✓  AND  D = ∅  AND  owner approves
        ▼
   ┌──────────┐
   │ approved │   TERMINAL  →  ship from here
   └──────────┘
```

### 4.3 Binding rules

These rules control behaviour. Read them before you drive any transition.

**R1 — T4 refuse-rule.** `begin` is defined only at `accepted`. If asked to implement a plan in any
other state, **refuse and say why**. An unreviewed plan is not implementable. Explicit owner
approval at T3, with Open Questions resolved to *None*, is the only door out of planning.

**R2 — T5 dual gate.** A phase exits only when **both** gates pass. The test gate is green,
**and** the owner answered the comprehension question correctly. Failing either gate is the T5'
self-loop: fix the code, or explain again and ask again. Never start phase `N+1` on an unverified
phase.

**R3 — Deferral is the owner's call, never the agent's.** The owner may defer a phase's
comprehension question. The owner may defer per phase, or as a standing order over a whole build
("build every phase continuously; I will do comprehension at review"). A deferred phase advances on
the test gate alone. Three limits hold:

- The agent still **asks** the question at the phase boundary, and **books it into the debt `D`** on
  the board. The question stays concrete and answerable later.
- **`test ✓` is never deferrable.** The test gate is green at every phase. No exception.
- The agent **never** defers on its own initiative. With no explicit owner order, R2 is in force.
  One exception exists: **§9, degraded mode**, where no owner is reachable to give the order. The
  debt `D` still grows, and the plan still cannot reach `approved`.

Deferral does not weaken the comprehension requirement. It **relocates** it. T9 requires `D = ∅`, so
every deferred question is answered before the plan is approved.

**R4 — T10 re-plan in place.** A design flaw found during a build sends the plan back to `proposed`.
Revise the same document — there is no `superseded` state. The revised plan may renumber its phases.
After re-approval, the build restarts at the phase the revised plan declares. Phases the re-plan
invalidated are gone, and any deferred questions they booked drop with them.

**R5 — `in-review` is a black box.** This file does not define the review procedure. Define one in
§10, or make the state a single owner check. Keep the state itself. T9 requires the owner's explicit
approval **and** `D = ∅`.

**R6 — `approved` is terminal. Shipping is after it.** Commit, merge, and deploy after `approved`.
The plan stays `approved`. There is no `shipped` state.

**R7 — Priority is not a transition.** Priority is a board column. A lower number means higher
priority. The scheduler is **non-preemptive**: change the active plan only at a phase boundary, at
`implemented`, or at `approved`. Never abandon a phase in the middle for a more urgent plan.

**R8 — The executive order.** A terse imperative from the owner ("implement it now") can compress
T3 and T4 into one event, with a standing deferral over every phase. Two conditions must **both**
hold:

1. The design fork is closed. Either the plan shows `OQ = ∅`, or the owner just picked an option
   from a menu.
2. The message is a direct, unambiguous order to build. Not a question. Not "what do you think".

If either condition is missing, **do not self-invoke this pattern**. That would break R3.

An executive order compresses the transitions. It never waives the plan document (§3.1) and never
waives the test gate.

---

## 5. Plain language — ASD-STE100

**Rule: write every plan document's prose in ASD-STE100 Simplified Technical English.**

ASD-STE100 is a controlled English standard from aerospace maintenance documentation. It exists so a
reader under load reads once and understands once.

### 5.1 What it means in practice

- Short sentences. One clause each.
- Active voice.
- Imperative mood for instructions.
- One approved meaning per word. Do not use one word for two ideas in one document.
- No chained subordinate clauses.
- No two independent clauses joined by a semicolon. Use two sentences.
- No long noun chains.

### 5.2 Scope

**Applies to:** the prose of a plan document — Summary, Context, Decision, Architecture, Non-goals,
Open Questions, Constraints. Also applies to a written answer to a genuine question from the owner.

**Does not apply to:**

- **Code identifiers, file paths, commands, and `file:line` citations.** These stay exact. The rule
  governs the prose around a fact, never the fact.
- **Board notes.** The board has its own dense, abbreviated convention. It serves scanning many rows
  at once. Do not rewrite board notes into STE-100.
- **User-facing published copy.** Marketing text, UI strings, and documentation for end users follow
  the product's voice, not this rule.

### 5.3 Not retroactive

The rule applies to new documents, and to any document you are already editing. Do not sweep an old
document into STE-100 only because the rule now exists. Touch an old document's register only when
you edit it for another reason.

### 5.4 The failure signature

Audit new prose against these four signs. Each one means the document is not well-formed:

- A long compound sentence.
- Passive voice where an actor exists.
- A trade-off buried in the middle of a paragraph.
- A word used with two different meanings in the same document.

**Where drift concentrates:** in text added fast, across several quick turns — an answered Open
Question, a scope change, a phase split. Re-read the **newly added** prose before you call the
document settled.

### 5.5 The structural companion

STE-100 controls the sentence. These five rules control the document. Use both.

1. **Concept before mechanism.** Say what the thing is before you say how it works.
2. **An ASCII diagram before any spatial or data-flow numbers.** The reader needs the shape before
   the measurements.
3. **Every "we chose X" names X's cost.** A decision with no stated cost is not a decision. It is an
   advertisement.
4. **Frame each Open Question as a decision the owner owes.** Give the options. Give each option's
   cost. Do not hide a fork inside prose.
5. **No dead taxonomy.** Do not list categories the document never uses again.

---

## 6. Reference attachment and grounding tiers

A plan makes claims about the domain. Each load-bearing claim needs a source.

### 6.1 The grounding tiers

Record which tier each source belongs to.

| Tier | What it is | How to use it |
|------|------------|---------------|
| **Tier 1** | A primary source the agent can open — a book in the repository, a specification, a paper. | Read the exact page. **Cite it by page or section.** Never paraphrase from memory. |
| **Tier 2** | A named authority the agent cannot open here — a standard, a well-known reference. | **Name it and cite it.** Never re-derive its result. |
| **Tier 3** | The project's own code, board, and memory files. | **Verify against the current tree** before you cite. A memory is a point-in-time observation, not live state. |

### 6.2 The well-formedness rule

**A plan that asserts a domain fact with no reference is not well-formed.** When you cannot back a
claim, do not assert it. Fold the gap into an Open Question.

This is the producer side. §6.3 is the consumer side.

### 6.3 The foundations document

For a project with a hard domain, keep one `FOUNDATIONS.md`. It holds the domain knowledge that
sharpens planning, before any code exists.

Write it as numbered **cards**. Each card has five parts:

| Part | Contains |
|------|----------|
| **Fundamental** | The transferable truth, grounded in a cited source. |
| **This project** | How this codebase realizes it. Verified against current code. Cite `file:line`. |
| **RFC hook** | **What a plan touching this concept must declare, and in which section.** |
| **Failure mode** | The specific mistake this card prevents. |
| **Source** | The tier and the exact citation. |

**The RFC hook is what makes reference attachment enforceable.** Without it, a foundations document
is a reading list. With it, a card states an obligation a plan must meet. The plan cites the card,
and the card says what the plan must declare.

Open the foundations document **first**, before you write a plan. Read the map, then read the one or
two cards your plan touches.

---

## 7. Knowledge keeping — five kinds of document, five rules

This is the part most projects get wrong. **The persistence rule differs per document kind.** Know
which kind you are editing before you edit it.

| Kind | Rule |
|------|------|
| **Plan / RFC** | **Current state.** It says what is true now, top to bottom. |
| **Board / index** | **Single source of truth for scheduling.** Update the row at every transition. |
| **Foundations** | **One home per fact.** Route to the fact; never copy it. |
| **Session log** | **History** — with one amendment rule. |
| **Memory leaf** | **One durable rule per file.** A point-in-time observation, not live state. |

### 7.1 Plan — current state, not a changelog

A plan document accumulates state across sessions. Do **not** keep that history inside the live
document as a stack of update banners and struck-through text.

- **One status section, dated.** Collapse every banner into one `## Status — latest (YYYY-MM-DD)`.
  State the current state, what shipped, where the build diverged from the plan, and the measured
  result. Put it at the top.
- **Delete struck-through text. Do not stack it.** Replace `old new` with `new`. If the change
  itself matters, write one line in the status section.
- **Resolve an Open Question in place.** A resolved question becomes its answer, or moves into the
  Decision section. It does not become a struck-through question with a note.
- **Trigger:** consolidate at any settle or ship boundary. Also consolidate the moment you notice
  two or more stacked banners. Do this without being asked.

The history is not lost. It is in version control, in the session logs, and in the review record.
Duplicating it inside the plan only rots — the old text and the new text drift apart, and the plan a
future reader needs is buried.

### 7.2 Board — the scheduling source of truth

One row per plan. Columns: **plan · status · priority · phase (`N` of `K`) · write-set ·
depends-on · collides-with · notes**.

- Update the row at **every** transition. A board that lags is worse than no board.
- Keep the debt `D` visible — the deferred comprehension questions, per plan.
- Before you trust a board row, **check it against version control**. The board records intent. The
  repository records fact. When they disagree, the repository wins, and you fix the board.
- The board is exempt from §5. Its dense style serves scanning.

### 7.3 Foundations — an orientation map, never a fact copy

When you consolidate content into an authoritative document, **do not copy facts it already owns**.

Inline an **orientation map** instead: what the thing is, plus routing pointers ("fact X → §N").
Leave every constant, formula, and line reference in its single home. Cross-link the source.

A second copy of a fact is a second thing to maintain. The two copies drift. The drift is silent.
This is exactly the failure an authoritative document exists to prevent.

**Also check the premise of an instruction to inline.** An instruction to "inline document X" claims
X holds current, non-duplicate content. Verify that claim first. When the claim fails — X is stale,
or the facts already live in the cards — say so plainly, ship the map, and offer the literal inline
if the owner still wants it.

### 7.4 Session log — history, amended for code drift only

Write one log per working session that changed code, reviewed code, or made a decision.

Name it `session_log_YYYYMMDD_<slug>.md`. Use eight sections:

1. **Initial Prompt** — the owner's exact words. Verbatim. No paraphrase. Misreading starts here.
2. **What The Agent Understood** — the mental model that drove the first attempt. Write it even when
   it was wrong. Especially when it was wrong.
3. **What The Agent Did** — actions in order. Decisions, not narration.
4. **What The Owner Changed Manually** — the owner's direct edits, at diff level. This section is the
   ground truth on what the right answer looked like.
5. **The Correct Understanding** — the real requirement, now visible. Write it so a future agent
   implements it correctly on the first try.
6. **Miscommunication** — the root cause of the gap between §2 and §5. Be specific. Write *None* when
   there was none.
7. **Next Steps** — the work that follows, in order. One line each, starting with a verb. Name the
   target file.
8. **What Worked** — non-obvious approaches the owner accepted without correction. Write *None* when
   the session was purely corrective.

**The amendment rule has two halves. Do not mix them.**

- **A later refactor made the log's code descriptions stale → amend §3, §4, §5, §7, §8 in place.**
  Rewrite the identifiers and signatures to the current names. The log's job is forward-looking. A
  future agent must read it and get the target state directly.
- **A later analysis corrected an earlier session's conclusion → record the correction forward, in
  the new log.** Do not rewrite the old conclusion. Keep §1 and §6 as written in every case. They
  are genuine history. Rewriting a past conclusion erases what was known when.

### 7.5 Memory leaf — one durable rule per file

A memory leaf holds one extracted fact or rule. One fact, one file.

```markdown
---
name: <short-kebab-case-slug>
description: <one line — used to decide relevance during recall>
metadata:
  type: user | feedback | project | reference
---

<The fact. For `feedback` and `project`, follow with the two lines below.>

**Why:** <what happened that produced this rule — quote the owner where possible>

**How to apply:** <the concrete trigger and the concrete action>
```

Types:

- `user` — who the owner is. Role, expertise, preferences.
- `feedback` — a correction, or a confirmed non-obvious approach. **Always include the why.**
- `project` — ongoing work, a goal, or a constraint the code does not record. Convert every relative
  date to an absolute date.
- `reference` — a pointer to an external resource.

Link related leaves with `[[other-leaf-name]]`. Link generously. A link to a leaf that does not
exist yet marks work worth doing. It is not an error.

**Write a feedback leaf whenever the owner corrects you, or confirms a non-obvious approach you were
not sure about.** This is the highest-value memory type. It is how the next session avoids this
session's mistake.

**Before you write, check for a leaf that already covers the fact.** Update that file instead of
creating a second one. **Delete a leaf that turns out to be wrong.** A wrong memory is worse than no
memory.

Do not save what the repository already records — the code structure, a past fix, the commit
history. If asked to remember one of those, ask what was non-obvious about it, and save that.

### 7.6 The memory index

`MEMORY.md` is the index. It is loaded into context at the start of every session.

- **One line per memory file.** A link, plus a hook of a few words.
- **Never put content in the index.** The index routes. The leaf holds.
- Group the lines by kind: session logs, feedback, project, reference.
- **Compact it as it grows.** Move old session-log lines into an archive file, and leave one line
  pointing at the archive. Keep the recent lines and every feedback line in the live index.

---

## 8. Update triggers — what to write, and when

Use this table at the end of every unit of work.

| Event | Update |
|-------|--------|
| A plan reaches any new state | The board row. Every time. |
| A phase passes both gates | The board row's phase number. |
| A phase advances on a deferral | The board row's debt `D`, with the exact question. |
| The owner answers an Open Question | The plan — fold the answer into the prose, present tense. Then delete the question. |
| A code change makes a plan's `file:line` wrong | The plan. Correct the citation. |
| A plan grows two or more stacked banners | The plan. Consolidate to one dated status section. |
| A domain fact is learned from a Tier-1 or Tier-2 source | The foundations document. Add or extend a card, with the citation. |
| A refactor makes an old session log's code descriptions stale | That log, in place — §3, §4, §5, §7, §8 only. |
| A later analysis corrects an earlier conclusion | The **new** log. Never the old one. |
| The owner corrects the agent | A `feedback_*.md` leaf, plus one line in `MEMORY.md`. |
| The owner confirms a non-obvious approach | The same. A confirmation is as valuable as a correction. |
| A session ends that changed code | A session log, plus one line in `MEMORY.md`. |
| A memory leaf is found to be wrong | Delete the leaf. Delete its index line. |

**Do the update in the same session.** A document you plan to fix later is a document that stays
wrong.

---

## 9. Degraded mode — when the owner is not present

This method assumes a human at every phase boundary. Sometimes there is none — a solo run, a
scheduled run, an automated pipeline. Keep the method. Degrade it in one specific way.

**What still holds, with no owner present:**

- **`test ✓` is never deferrable.** The test gate is green at every phase. This is the one invariant
  that survives the owner's absence. It is why §1 requires a runnable test command.
- **The plan document is still written first.** No plan, no build.
- **Every phase still poses its comprehension question**, and books it into the debt `D`. Write the
  question down. Do not invent an answer. **The agent never answers its own comprehension
  question** — a self-answered question tests nothing.
- **T9 still requires `D = ∅`.** With no owner, the plan **cannot reach `approved`**. It stops at
  `implemented` or `in-review`, and it waits.

**What changes:** the agent may run T5 on the test gate alone, and continue through the phases. This
is the standing deferral of R3, applied by necessity rather than by order. Record it on the board as
such, so the owner sees on return that every comprehension question is still owed.

**What the agent must never do in this mode:** fire T3, fire T9, close an Open Question by choosing,
or mark a plan `approved`. An absent owner is not a silent yes.

---

## 10. Project section — ride

Everything above is the method, copied unchanged. Everything here is this project.

### What ride is

A small pure functional language over numbers. A program declares a STATE record and a set of
functions. Evaluating the entry function against one STATE record yields **one number**.

There is nothing else in the language: no strings, no collections, no effects, and no state
carried between calls. A program is a mathematical function, and the host owns everything else.

```
state { s, t }                      the record the host supplies
let phase x = 0.6133x - 220.788     a named function
let main =                          the entry point, no parameters
    if s < 345 then 350.1
    else s + 5 - cos(phase(s))
```

**This is not an animation system and not a renderer.** It computes numbers. A host that wants
to animate something calls it once per frame per property and does the animating itself.

### Paths

| what | where |
|------|-------|
| plans (RFCs) | `.claude/tasks/` |
| the board | `.claude/tasks/_index.md` |
| the plan template | `.claude/tasks/_template.md` |
| memory (`<memory>` in §1) | `.claude/memory/` |

### The test command

```
Test (front end):   npm test                 # vitest, test/
Test (runtime):     npm run test:runtime     # cargo test, runtime/
Test (both):        npm run check            # the gate; run this before any `verified`
Differential:       node scripts/differential.mjs && npm run test:runtime
Typecheck:          npx tsc --noEmit
Codegen:            npm run langium:generate # after ANY edit to src/ride.langium
Build (TS):         npm run build
Build (wasm):       npm run build:wasm
```

`npm run check` is the gate the lifecycle depends on. It must be green at every phase. The
runtime half needs no network and no npm: `cargo test` runs against an empty dependency graph.

**After editing `src/ride.langium` you must run `npm run langium:generate`.** The generated
parser and AST in `src/generated/` are committed, and a stale generated directory makes the tests
pass against a grammar that is no longer the source of truth.

### Architectural commitments

These are load-bearing. Do not design around one for convenience. If a shortcut trades one of
them for simpler code, name the trade-off and let the owner decide.

1. **The runtime has no dependencies and no allocator.** `runtime/Cargo.toml` lists
   `wasm-bindgen` only behind the optional `wasm` feature. The evaluator runs on three
   fixed-size arrays and allocates nothing per call. Do not add a crate to the runtime, and do
   not introduce a heap value.

2. **The runtime never parses.** The Langium front end in `src/` is the only parser. Keeping it
   out of the runtime is what lets the runtime stay dependency-free and small.

3. **Nothing is evaluated until both proofs pass.** `runtime/src/verify.rs` holds them:

   * **Proof A** bounds the operand stack per function. A single accumulating scan over the
     instruction array is sound only for straight-line code, so Proof A keeps a height table
     and checks it **at each jump target** rather than accumulating. This is the rule a
     WebAssembly validator applies at a block boundary.
   * **Proof B** bounds the frame array and the return stack. It builds the call graph, rejects
     any cycle, and takes the heaviest root-to-leaf path through the resulting DAG.

   `Verified` can only be built by running both. The evaluator accepts nothing else, and it
   indexes its arrays with no run-time guard — the proofs are the guard.

4. **The language is first-order.** A function is never a value. There is no lambda, and a
   parameter is always a number. This is what makes every call direct and saturated, and it is
   also what makes implicit multiplication unambiguous: application is always written
   `f(a, b)`, so a number next to a term can only mean multiplication.

   **Adding curried application would break both.** It would require a closure representation
   and it would make `15exp(x)` ambiguous.

5. **Recursion is forbidden, and the ban is checked twice.** The compiler rejects a cycle by
   name, because a name is more use than an opcode index. `verify` rejects it again, because
   bytecode can arrive without passing through the compiler. The ban is not a style rule: an
   acyclic call graph is the premise of Proof B.

6. **Jumps are forward-only.** The language has no loop form, so no legal program needs a
   backward jump. `verify` rejects one. This is a fail-loud requirement, not a simplification:
   a backward jump in untrusted bytecode is an infinite loop in the evaluator.

7. **The compiler does not simplify the arithmetic.** It lowers and emits; the shape the author
   wrote is the shape that runs. An algebraic simplifier reassociates, `f32` addition is not
   associative, and a reader comparing source to behaviour would then have to account for the
   difference. If a simplifier is ever added, the gate is a bit-for-bit comparison of both paths
   over every example.

8. **Every error is a compile-time or verify-time error.** The one exception is
   `EvalError::StateTooShort`, which is a property of the call rather than of the program.
   Errors leave the runtime as stable string codes, never as a panic and never as a formatted
   message.

### The open decision — do not settle this by implementation

**The language has one number type.** `terminal NUMBER` in `src/ride.langium`, and `f32`
throughout the runtime.

Adding a separate `int` is **a one-way door**, and it is the owner's decision:

* It changes every arithmetic operation, every literal, and the whole error set.
* `f32` division by zero yields an infinity. **`i32.div_s` traps**, on divide-by-zero and on
  `MIN / -1`. So `int` introduces a trap path into per-frame code, which commitment 8 forbids.
* Three answers exist, and one must be chosen before the value representation is fixed: forbid
  integer division, define it to return a value, or prove the divisor non-zero at compile time.
* ML-style distinct operators per type (`+` against `+.`) is one way to get "no silent
  coercion", and the surface syntax leaves room for it.

Until the owner decides, keep one number type. Do not add `int` as a convenience.

### Surfaces (the write-set legend)

| id | surface | files |
|----|---------|-------|
| `G` | grammar | `src/ride.langium` — **requires `npm run langium:generate` after any edit** |
| `g` | generated parser | `src/generated/**` — machine-written; never hand-edit |
| `C` | compiler | `src/compile.ts` — the five stages: collect, check, acyclic, emit, link |
| `F` | front-end API | `src/index.ts`, `src/evaluate.ts` |
| `R` | runtime: ops | `runtime/src/op.rs` — the instruction set |
| `V` | runtime: proofs | `runtime/src/verify.rs` — Proof A and Proof B |
| `E` | runtime: evaluator | `runtime/src/eval.rs` — the dispatch loop |
| `T` | tests | `test/**`, `runtime/tests/**` |
| `D` | differential | `scripts/differential.mjs`, `runtime/tests/differential.rs` (generated) |
| `X` | examples | `examples/*.ride` — the differential harness sweeps these |

**The coupling that matters most:** adding an instruction touches `R`, `V` and `E` together.
A new variant in `op.rs` is a compile error in `verify.rs` by design — the matches there are
exhaustive with no wildcard, so the prover cannot silently miss it. It owes a stack effect in
Proof A **and** the same effect re-derived for untrusted input. `Call` is the one to watch: its
effect is "pop arity, push one" **only because** Proof B accounts for the callee's frame
separately.

A new instruction also owes an arm in `src/evaluate.ts`, or the two evaluators diverge and the
differential test is the thing that will tell you.

### Review procedure

Owner check only. There is no separate review workflow in this project yet.

Before presenting any change, run `npm run check`. A change to the instruction set, the proofs
or the evaluator additionally owes a regenerated differential test:

```
npm run build && node scripts/differential.mjs && npm run test:runtime
```

### Craft rules

**Rust.**

* `#![forbid(unsafe_code)]` stays in `runtime/src/lib.rs`.
* No panic on the evaluation path. A panic there means a proof is wrong — fix the proof.
* The matches in `verify.rs` are exhaustive with no `_` wildcard. Keep them that way: the
  compile error on a new variant is the mechanism, not an inconvenience.
* Every `VerifyError` has a distinct stable `code()`. A test asserts they do not collide,
  because the codes cross the host boundary.

**TypeScript.**

* The compiler returns diagnostics; it never throws on bad input. A throw means a compiler bug.
* A diagnostic says what is wrong **and** what the rule is. `` `g` is a function, so it cannot
  be used as a value. ride is first-order: write `g(…)` to call it. `` is the standard to match.
* `src/evaluate.ts` rounds every arithmetic result through `Math.fround`, because the runtime
  works in `f32`. Without it the two evaluators drift.

**Numbers.**

* The transcendentals are **not** guaranteed bit-identical between the two evaluators.
  `Math.cos` computes in `f64` and is rounded; Rust's `f32::cos` computes in `f32` throughout.
  The differential harness uses a relative tolerance for this reason, and the reason is written
  at the top of the generated file. Do not tighten it to exact equality.
* The arithmetic, comparison, branch and call instructions **are** bit-exact. A difference there
  is a fault, not rounding.
* When asserting a numeric result in a test, state the measured value and why the tolerance is
  what it is. A bare `toBeCloseTo` hides which digit moved.

---

## 11. Behavioural layer

The method above covers planning, the lifecycle, prose and knowledge keeping. It does not cover
how to write the code itself. This section does. It is adopted from Andrej Karpathy's
`CLAUDE.md`, whose own header says to merge it with project-specific instructions.

Where it meets §1–§10, §1–§10 wins — those are this project's binding rules. Everything here is
about conduct inside a single edit.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

### 11.1 Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### 11.2 Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### 11.3 Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### 11.4 Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.
