# Work Skills — Exit Code Namespace

Exit codes for the `skills/work/scripts/` helpers that expose a stable
machine-readable contract. The codes are declared as `readonly E_*=NN`
constants in the owning script (the source of truth); this document is derived.

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
