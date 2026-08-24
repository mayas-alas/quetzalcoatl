# GAUNTLET — the adversarial build loop

The loop is small; the discipline is in the setup, and most failures come from
skipping step one.

## One round

1. **Write the bar before anything exists.** A bar written afterwards is a
   description of the artifact. A usable bar is **named** (a specific thing),
   **fetchable** (the critic can inspect it without asking the builder) and
   **losable** (you can describe what failing looks like). Record it in the
   board row or a `bar.md`. Blind comparison is the sharpest form: put the
   artifact and the reference side by side with labels stripped and pick.
   A bar renegotiated after the artifact exists is not a bar.
2. **The builder produces a real artifact** on its own `agent/<name>/<slug>`
   branch. Not a plan, not a diff summary — something that runs, renders or
   fails on its own.
3. **Package the evidence.** Commit hashes, a git range, test output, the
   boundaries you exercised. A builder's self-report is a hypothesis, not
   evidence.
4. **Hand a fresh critic the artifact and the bar, and nothing else.** No
   builder history, no rationale, no list of limitations. Those inputs talk a
   critic out of a rejection.
5. **The critic reproduces, then rules.** A claim it cannot reproduce did not
   happen. The verdict is a binary `PASS`/`REJECT` against the bar plus one
   **largest gap**; smaller gaps are recorded and deliberately left.
6. **The largest gap goes back** to the builder on the same branch. Smaller
   gaps are recorded, not fixed this round.
7. **Stop on a rule written in advance**, not on a feeling.

## Verdict must be a file

```text
# verdict.txt  (or the board row verdict cell)
VERDICT     PASS | REJECT
BAR MET     yes | no
REPRO       how the critic reproduced the claims
LARGEST GAP one gap, named and reproducible (empty if PASS)
SMALLER     recorded, not fixed this round
NOTE        bar stays as written; do not renegotiate to pass
```

## Stop rules (chosen per ticket before round 1)

- The artifact wins the blind compare twice, with two different critics.
- The largest gap for two consecutive rounds is cosmetic — the bar is met and
  the critic is inventing work.
- The budget (time-box) is spent; log it honestly.
- The same gap returns a third time — that is oscillation; the bar and the
  artifact are in conflict. Escalate to the orchestrator rather than loop.

Never stop on a fixed round count: three rounds is a schedule, not a signal.

## Critic independence

- **Fresh context**: the critic does not inherit the builder's session. Shared
  context is shared assumptions; a critic grading against the builder's mental
  model grades against the wrong thing.
- **Judge the artifact, not the story**: the critic must reach the same verdict
  with the builder's report deleted.
- **Real authority to reject**: if nothing has been rejected recently you are
  running an approval queue, not a gauntlet.
- **Anchor to facts**: a test suite, the compilers, the change-scoped
  validators. Reviewer opinion is not evidence; a failing check is.
- **Majority-refute panels**: if any critic produces a reproducible failure the
  artifact loses, even if others pass. An approval is only the absence of a
  demonstrated defect. Different critics look for different things (security,
  performance, reproduction, claims-vs-artifact), not the same blurry lens.

## Loop-guard (anti-buggy-loop)

- "Try again with nothing changed" is prohibited. A reintent must differ.
- Never soften a bar to pass after the artifact exists.
- No self-review: a builder who judged its own work did not run a gauntlet.
- A `REJECT` without a named reproducible gap is not a rejection. Three vague
  ones in a row mean the bar is unusable — fix the bar, not the loop.
- On a guard trigger: pause, record a correction in `BOARD.md`, then
  `parked` (reason) or `blocked` (blocker + needs) and notify the orchestrator.
  Do not re-claim the same ticket without a materially new plan.

## Change-scoped verification

Every candidate that reaches `PASS` must, before merge to `hot`, pass the
change-scoped validators that cover the touched paths (e.g.
`tools/check.ps1 -SourceOnly` or the relevant `tools/validation/*.py`), with
the exact command and its exit code recorded as evidence.