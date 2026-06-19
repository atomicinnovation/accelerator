# Work Skills — Exit Code Namespace

Exit codes for the `skills/work/scripts/` helpers that expose a stable
machine-readable contract. The codes are declared as `readonly E_*=NN`
constants in the owning script (the source of truth); this document is derived.

## Shared bridge taxonomy — `work-item-bridge-codes.sh`

The three work → integrations bridges (create / fetch / update) and the
`work-item-push-decide.sh` decision seam share **one** `E_DISPATCH_*` namespace,
defined once in the sourced `work-item-bridge-codes.sh` (the single owner) so the
70/71/72/73 values cannot drift between bridges:

| Code | Name                       | Meaning                                                                                                                                                          |
|------|----------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 70   | `E_DISPATCH_RETRYABLE`     | Failure provably **before** any remote mutation (arg/validation/auth/connect). For a **read** bridge this just means "read failed / degrade". **Safe to retry.** |
| 71   | `E_DISPATCH_TERMINAL`      | Failure **at or after** a mutation (response lost/invalid). **NOT safe to auto-retry.** Read bridges never emit this.                                            |
| 72   | `E_DISPATCH_NOT_AVAILABLE` | Tracker recognised but the operation is not built yet (`trello` / `github-issues`).                                                                              |
| 73   | `E_DISPATCH_UNRECOGNISED`  | `<sys>` not in `{linear, jira, trello, github-issues}`, or empty. **Fail closed.**                                                                               |

## `work-item-create-remote.sh` — push dispatcher taxonomy

The dispatcher maps each integration's native exit codes into one uniform
taxonomy, so `/create-work-item` branches on these codes alone and never on
tracker-specific output. The retryable/terminal split is the safety-critical
distinction: the remote create is **non-idempotent**, so a terminal outcome
must never be retried.

| Code | Name                        | Meaning                                                                                       |
|------|-----------------------------|-----------------------------------------------------------------------------------------------|
| 0    | —                           | Success; the bare validated identifier is on stdout                                           |
| 70   | `E_DISPATCH_RETRYABLE`      | Failure provably **before** the remote mutation was sent (arg/validation/auth/4xx-reject/connect-refused). **Safe to retry.** |
| 71   | `E_DISPATCH_TERMINAL`       | Failure **at or after** the mutation (request sent; response/identifier lost or invalid). A remote issue **may already exist** — **NOT safe to retry.** |
| 72   | `E_DISPATCH_NOT_AVAILABLE`  | `trello` / `github-issues`: no create path is built yet (work items 0049 / 0050).             |
| 73   | `E_DISPATCH_UNRECOGNISED`   | `<sys>` is not one of `{linear, jira, trello, github-issues}`, or is empty. **Fail closed.**   |

Per-integration mapping (native code → taxonomy):

- **Linear** (`linear-create-flow.sh` no-file mode): `108 E_CREATE_PRE_SEND` →
  70; `109 E_CREATE_POST_SEND` → 71; any other non-zero → 71 (conservative).
- **Jira** (`jira-resolve-fields.sh` + `jira-emit-key.sh` + `jira-create-flow.sh`
  + `jira-request.sh`): provably-no-create codes (`100–108`, `11/12/13/14/15/17/19/22/34`)
  → 70; bad-response (`16`), 5xx (`20`), connect/DNS/timeout (`21`), and any
  unrecognised code → 71.

Identifier-format validation is **per-tracker** (each integration validates its
own native shape). The dispatcher applies only a tracker-agnostic safety check
(reject control characters, newlines, a leading `---`, and a leading `#` comment
trigger; permit `/`, `#`, `@` mid-token) before passing the identifier through;
an unsafe identifier is reported as `71` (the issue may exist).

## `work-item-push-decide.sh` — decision seam (no numeric codes)

`work-item-push-decide.sh` returns `0`/`2` (success/usage) and prints one of the
following **action keywords** on stdout, derived from the dispatcher code, the
attempt number, and a post-dispatcher write-result flag:

| Action          | When                                                                          |
|-----------------|-------------------------------------------------------------------------------|
| `write-once`    | dispatcher `0` and the local Write has not failed                             |
| `retry`         | dispatcher `70` on the first attempt (offer exactly one retry)                |
| `local-save`    | dispatcher `70` after the retry is exhausted, or `72`/`73`                     |
| `loud-terminal` | dispatcher `71`, or dispatcher `0` but the single local Write then failed      |

## `work-item-fetch-remote.sh` — read bridge

The read counterpart to the create bridge. `search`/`show` dispatch to the active
tracker; `search --keys k1,k2,…` returns a tracker-agnostic
`{found,absent,indeterminate}` map (per-key markers, exit `0`). A read mutates
nothing, so failures collapse to the shared `70` (the caller degrades to
presence-only); `72`/`73` as above. An **incomplete** fetch (jira chunk/page-cap
hit, linear `truncated:true`) marks the un-confirmed keys **indeterminate**, never
absent — "remote-absent" is only ever drawn from a provably complete fetch.

## `work-item-update-remote.sh` — update bridge

Replaces an already-synced item's whole content (summary/title + body). Maps each
tracker's update-flow native codes into the shared taxonomy: provably-pre-mutation
codes → `70`; at/after-mutation or uncertain (5xx, dropped/200-body GraphQL error,
connect) → `71` (never auto-retried — the response is uncertain; a whole-item
update is idempotent, so the hazard is uncertainty, not double-apply).
