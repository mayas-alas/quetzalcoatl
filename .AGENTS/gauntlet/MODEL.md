# MODEL — roles, states, badges, corrections

Vocabulary is shared across `README`, `GAUNTLET`, `BOARD` and `AGENTS.md`.

## Roles

| Role | Grade | Function |
|---|---|---|
| `orchestrator` | 3 · direction | Assigns tickets, arbitrates bar disputes, is the final loop-guard authority. Owns `.AGENTS/gauntlet/**`, `.AGENTS/TRACKER.md`. |
| `architect` | 3 · direction | Freezes scope and **writes the bar before the builder starts**. Owns contracts and path boundaries. |
| `builder` | 1 · execution | Produces the real artifact on its own `agent/<name>/<slug>` branch. Merges the execution grade of the former `doer`/`sr`. |
| `critic` | 2 · quality | Fresh-context blind judge: `PASS`/`REJECT` plus one largest gap. Merges the former `verificador`/`juez`. |

Guarantees

- **Separated judgment**: the critic of a ticket is never its builder, has
  fresh context, and judges the artifact, not the builder's account of it.
- **Contained ownership**: a path has one owner at a time; an agent works only
  its claimed ticket and its own branch.
- An agent can promote grade only by orchestrator decision, never by
  self-declaration.

## Ticket states

| State | Meaning | Required |
|---|---|---|
| `queued` | Waiting for an owner. | — |
| `claimed` | A builder took it and named its branch. | `agent`, `branch`, `bar` |
| `building` | In progress; checkpoint active. | `checkpoint`, `updated_at` |
| `review` | Handed to a critic for the gauntlet. | artifact + evidence packed |
| `passed` | Critic verdict `PASS`; change ready for `hot`. | `verdict=PASS`, evidence |
| `done` | Merged into `hot` → `master`. | merge commit, evidence |
| `rejected` | Critic verdict `REJECT`; one largest gap back to builder. | `verdict=REJECT`, `largest_gap` |
| `blocked` | Held by a concrete blocker. | `blocker`, `needs` |
| `parked` | Returned to `queued` by the loop-guard with a reason. | `guard_reason` |

Transitions are exactly `queued → claimed → building → review → passed →
done` (or `rejected` back to `building`, or `blocked`/`parked` from any
active state).

## Badges — no XP

Quality is evidenced, not tallied. Badges are grant-records awarded by the
orchestrator on confirmed history; there is no point arithmetic.

| Badge | Condition |
|---|---|
| `clean-run` | Gauntlet `PASS` on the first round (no rejection). |
| `one-shot` | 3 consecutive clean runs. |
| `bug-slayer` | A critic caught a real bug the builder then fixed. |
| `critic-proof` | Survived a `REJECT`, fixing the named gap reproducibly. |
| `unblocker` | Wrote a `blocker`+`needs` that unblocked another ticket. |

## Corrections

Every rejected round and every `parked`/`blocked` decision is recorded in the
`## Corrections` section of `BOARD.md` *before* retrying, with the trigger.

```markdown
## CORR-<NNN> <ticket> — <symptom>
- Date: YYYY-MM-DD UTC
- Symptom: what failed, observable.
- Root cause: why it happened.
- Fix applied: what changed (paths).
- Prevention: what stops a repeat (check/gate).
- Status: fixed | parked | superseded
```

## Writing conventions

- One vocabulary everywhere; states verbatim.
- Dates in UTC (`YYYY-MM-DD`); branch names in `agent/<name>/<slug>`.
- One state-file change → one commit; never hand-rewrite history.