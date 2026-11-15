---
type: "pr-review"
id: "178-review-2"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-15T14:29:51+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:178"
relates_to: ["pr-review:178-review-1"]
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "usability", "security", "documentation"]
review_number: 2
pr_number: 178
tags: []
last_updated: "2026-11-15T14:29:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #178 - [0319] Let work create take an explicit work item ID

**Verdict:** COMMENT

The response to the first review is thorough. Nearly all of review 1's findings are fixed, and the core holds up:

- The ID grammar does split a whole value at most one way.
- The taken rule and allocation read the same holders.
- The create-lock ordering is shared through `refuse_requested_id`.
- The visualiser keeps serving when it cannot read the scheme.
- The retirement and run-plan decoders are hardened.

Most of what remains comes from the response's own new **convergence and well-formedness gate**. It disagrees with re-key about which items are "in shape", so some `E_REKEY_REQUIRED` refusals cannot be cleared by the remedy they name. It blocks read-only commands that skills call without handling exit 4. It has no upgrade path in the CHANGELOG. Its logic is built twice. No lens found anything critical.

### Cross-Cutting Themes

- **The convergence gate and re-key disagree** (flagged by: Correctness, Compatibility, Usability). They disagree in four cases:
  - `is_converged` compares literal pattern text case by case, while re-key's `is_in_target_shape` ignores case.
  - `classify` returns `TrackerKey` whenever `id == external_id`, before checking shape, so the documented "local and remote IDs coincide" setup is never converged. Re-key then renumbers it.
  - Convergence accepts any key, but re-key folds every key onto `work.key`.
  - Unclassifiable IDs get pointed at a re-key that refuses them.

  Result: the remedy the refusal names either does nothing, or does more than the refusal said. Use one definition of "in shape" for both.
- **Read-only commands now refuse, and their callers don't know** (flagged by: Usability, Compatibility, Documentation). `list`, `resolve`, `id check` and `promote` exit 4 over an off-pattern or malformed corpus. Several skills don't handle that:
  - `create-work-item`'s resolver table
  - `update-work-item`
  - `review-work-item`
  - `list-work-items`, which still claims to be "resilient"
  - `sync-work-items`, which finds files through `resolve`

  `sync` is exempt to avoid a deadlock, but the sync skill reaches files through `resolve` and `promote`, which are not exempt.
- **Upgraders get no path** (flagged by: Documentation, Compatibility). `### Migrations` has no entry for `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`, and the existing entry still calls re-keying optional. The configure skill still says width changes are "absorbed" and that skipping 0002 keeps allocation working, but under a `{key}` pattern both now refuse.
- **Corpus-trust policy built twice** (flagged by: Architecture, Code Quality). `work-cli/src/convergence.rs::refuse_diverged_corpus` and the visualiser's `server.rs::unconverged` run the same two steps independently. The divergence half lives under `rekey`. The dry-run and the real create still order the pre-lock steps separately.
- **Exit code read from message text** (flagged by: Architecture, Code Quality, Usability). `of_failure` maps exit 4 by checking that the message starts with `E_ID_TAKEN:`, even on paths that hold the typed refusal. The contract "a script tells a bad ID from a taken one by exit code" also no longer holds, because corpus refusals exit 4 too.
- **Read-only consumers handle a bad scheme differently** (flagged by: Architecture, Compatibility, Test Coverage). The visualiser degrades, but `corpus linkage extract` now fails for every doc type. That behaviour is untested and not in the CHANGELOG.

### Tradeoff Analysis

- **Integrity vs availability**: refusing `list` and `resolve` over an untrusted corpus keeps wrong IDs out of every reader. But it removes the read-only commands a user needs to diagnose the problem, and it strands skills mid-flow. Recommendation: keep the hard refusal for commands that allocate or write (`create`, `create-batch`, `id next`, `promote`). Let `list`, `resolve` and `id check` warn on stderr instead, or at least teach every calling skill the exit-4 branch.
- **Typed domain vs incremental fix**: Architecture and Code Quality would move the trust policy into one `work`-level operation, possibly a `TrustedCorpus` type. That is a sizeable refactor late in a large PR. The minimum is one shared function both adapters call.

### Strengths

- ✅ The ID grammar is sound. A literal after `{key}` cannot start with a digit, and a literal before `{number}` cannot end with one, so whole-value walks split one way at most. That goes further than the exhaustive test shows.
- ✅ Review 1's findings are largely fixed:
  - live linkage reads through the scheme
  - `scheme_from_settings` is the only way to build a scheme
  - `IdReadings` replaces the repeated `is_tracker_owned` branching
  - `KeySource` and `HeldPrefix` replace stringly or arithmetic code
  - `E_ID_TAKEN` exits 4
  - `work id` shares one `Error:` vocabulary
  - the retired verbs print `E_VERB_RETIRED`
  - an other-key ID now needs confirmation
- ✅ `refuse_requested_id` owns the pending-push-then-taken order for both the dry-run and the real create, and the real create runs it under the create lock.
- ✅ Allocation and the taken rule read the same holders, so allocation never hands out an ID `--id` would refuse. The arithmetic is safe throughout (`checked_pow`, `saturating_add`, width cap 19).
- ✅ The visualiser's identity-key index is deterministic: the lowest path wins whatever the read order, and the key is handed on correctly when its holder is removed.
- ✅ The visualiser degrades instead of refusing to boot. `problem` is an additive `skip_serializing_if` field, and the paths in it are relative to the root and escaped.
- ✅ Hardening:
  - retirement records refuse IDs outside the alphabet
  - `recovery_dir` must be `retirement-recovery/<one component>`, tested with `../../etc`, absolute and bare paths
  - each tracker validates its own key shape (`TrackerKeys`), and a Linear key can't hold `_`
- ✅ The refusals follow the code / what / remedy shape and escape hostile IDs.
- ✅ Test coverage is strong:
  - `cli_rekey_required.rs` runs every guarded command against one diverged fixture, checking the exit code, the exact message and that nothing changed
  - a cross-check pins convergence ≡ re-key-leaves-it under `work.key`
  - the coverage of the deleted `corpus-adapters` suites is re-homed
  - the eval guard is a pure function, tested synthetically
- ✅ `bin/accelerator` pins the launcher to the plugin version, so skills and the binary can't drift apart. The retired verbs never shipped in a stable release.

### General Findings

- 🔵 **Usability**: `E_REKEY_REQUIRED` points at `/rekey-work-items` even when the named item is unclassifiable (an `id` with no number), which re-key then refuses with `E_REKEY_UNCLASSIFIABLE_ID`. Name the manual remedy for unclassifiable items.
- 🔵 **Documentation**: `update-work-item`, `review-work-item` and `list-work-items` call `work resolve` or `work list` but don't document the exit-4 refusals. `list-work-items` also still says the CLI reads only `*.md`.
- 🔵 **Compatibility**: review 1's 0002 allocation-jump guidance was added, but under the convergence gate a legacy item that carries an `id` now blocks allocation instead.
- 🔵 **Security**: IDs are validated where they are decoded, not where they are written. `quoted_list`, `set_scalar` and `rewrite_review_link` interpolate IDs into double-quoted YAML without escaping. Use a type that guarantees the alphabet, or an escaping writer.
- 🔵 **Architecture**: `migrate-adapters/tests/common/mod.rs::WorkIdentity` is still a copy of the shipped `migrate-cli` adapter. Move the adapter somewhere both can import.
- 🔵 **Architecture**: `rekey::convergence` and `well_formed` import `retirement::relative_to` just to show paths. Move the helper to a neutral module.
- 🔵 **Code Quality**: work-cli's `convergence` module and `Diverged` also carry `E_WORK_ITEM_MALFORMED`. A name such as "trusted corpus" would cover both checks.
- 🔵 **Test Coverage**: `--project ""` → `E_WORK_KEY_INVALID` is tested only at the unit level. Add `""` to the hostile-project CLI cases.

### Additional Findings

- 🔵 `cli/work-cli/src/list.rs:585` — Read-only `list` refuses outright over an off-pattern corpus (Usability; folded into the `resolve.rs` comment)
- 🔵 `cli/work/src/well_formed.rs:75` — `E_WORK_ITEM_MALFORMED` names only the first malformed file, so fixing a corpus takes one re-run per file (Usability)
- 🔵 `cli/work-cli/src/exit_codes.rs:206` — Exit 4 no longer tells a script that an ID is taken (Usability)
- 🔵 `cli/visualiser/server/src/server.rs:55` — `problem` is invisible in the UI and fixed at boot, so it goes stale after a re-key (Usability)
- 🔵 `cli/work/src/rekey/mapping.rs:281` — Re-key to `{tracker}` checks the ID alphabet only for unsynced items (Security)
- 🔵 `cli/work/src/retirement.rs:267` — The `IdNotOnePathComponent` message prints the hostile ID unescaped before its escaped copy (Security)
- 🔵 `cli/work-cli/src/convergence.rs:32` — The corpus-trust policy lives under `rekey`, and each command opts in separately (Architecture)
- 🔵 `cli/visualiser/server/src/config.rs:28` — Scheme state is held as two parallel `Option`s, and `None` means opposite things in `Config` and `WorkItemConfig` (Architecture, Code Quality)
- 🔵 `cli/corpus-cli/src/main.rs:101` — `corpus linkage extract` now fails on a bad work scheme for every doc type, untested and not in the CHANGELOG (Architecture, Compatibility, Test Coverage)
- 🔵 `cli/work-cli/src/create.rs:669` — The dry-run and the real create still order the corpus checks and admission separately, and the real create reads the scheme twice, hidden by a `Once` guard (Architecture, Code Quality)
- 🔵 `cli/work/src/id_scheme.rs:141` — Tracker keys are stored twice and need a second construction step, and ownership branching still sits outside `IdReadings` (Architecture, Code Quality)
- 🔵 `cli/tracker/src/lib.rs:909` — Two separate matches on integration names (Architecture)
- 🔵 `cli/work/src/id_pattern.rs:511` — `segments_without_key` drops the wrong literal for `{number:04d}-{key}v` (Correctness)
- 🔵 `cli/visualiser/server/src/indexer.rs:1039` — An identity-key handover leaves `reviews_by_target` pointing at the old holder (Correctness)
- 🔵 `cli/visualiser/server/src/server.rs:82` — The visualiser degrade wiring and its malformed-item branch are untested end to end (Test Coverage)
- 🔵 `cli/work/src/well_formed.rs:84` — The path-order sort and the case-insensitive slug match both survive mutation (Test Coverage)
- 🔵 `cli/visualiser/server/src/compose.rs:77` — An error arm that can never run, and `SchemeError::Config` quietly treated as a degrade (Code Quality)
- 🔵 `cli/work/src/id_scheme/tracker_ids.rs:50` — `legacy_prefix` and the ID-alphabet predicate are each defined twice (Code Quality)
- 🔵 `cli/work/src/id_scheme/tracker_ids.rs:86` — `filename_identity` returns a bare tuple and works out its own `+1` slug offset (Code Quality)
- 🔵 `cli/work/src/well_formed.rs:17` — `MalformedItem` can represent impossible states, hidden behind a wildcard `Display` arm (Code Quality)
- 🔵 `cli/work-cli/src/id/check.rs:21` — The doc comment on `RunOutcome::Refused` is stale, here and in `create.rs` and `create_batch.rs` (Code Quality)
- 🔵 `cli/work-adapters/src/config.rs:252` — `work.integration` is read twice, and `KeyRequired` still hard-codes setting names (Code Quality)
- 🔵 `cli/visualiser/server/src/indexer.rs:471` — The identity-key rename left comments over 80 characters (Code Quality)
- 🔵 `cli/work-adapters/src/retirement_records.rs:96` — The hardening validates strings with booleans instead of parsing into types (Code Quality)
- 🔵 `CHANGELOG.md:243` — The `E_WORK_ITEM_MALFORMED` entry gives no remedy (Documentation)
- 🔵 `CHANGELOG.md:202` — The Breaking entry describes an `E_ID_TAKEN` exit 1 that never shipped (Documentation)
- 🔵 `skills/config/configure/SKILL.md:711` — The configure skill still says width changes are absorbed (Documentation)
- 🔵 `skills/config/configure/SKILL.md:811` — The 0002 skip guidance describes allocation that convergence now refuses (Documentation, Compatibility)
- 🔵 `cli/work-cli/src/cli.rs:239` — The `work id check` help leaves key-case restoration out of `corrected` (Documentation)
- 🔵 `docs-site/src/content/docs/guides/sync-work-items.mdx:237` — "Every other work command" overstates which commands refuse (Documentation)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/work/src/rekey/convergence.rs:26-32` — Convergence and re-key disagree about which IDs are in shape, so some refusals can't be cleared (Correctness, Compatibility)
**Severity**: major | **Confidence**: high | **Lens**: correctness, compatibility

🟡 **Correctness**

`is_converged` requires `is_any_key_id_token`, which reads through `read_exact` with exact literal case. Re-key keeps an item when `is_in_target_shape` holds, and that compares the rendered ID with `eq_ignore_ascii_case` (`rekey/sequence.rs:97`; `sequence.rs` even tests that `V001` under `v{number:03d}` stays in target shape). So these items are not converged, yet re-key leaves them untouched:
- `id: "V001"` under `v{number:03d}`
- `pp-0042` under a literal `PP-{number:04d}`
- `PPV001` under `{key}v{number:03d}`

`create`, `id next`, `id check`, `list`, `resolve` and `promote` all exit 4 with `E_REKEY_REQUIRED`, and the `/rekey-work-items` run the message prescribes changes nothing.

🟡 **Compatibility**

Under a local pattern, `is_converged` also requires `classify` to return `Sequenced`. But `classify` returns `TrackerKey` first whenever `id` equals `external_id` (ignoring case; `rekey/classify.rs:22-27`). So `ENG-1234` with `external_id: "ENG-1234"` under `{key}-{number:04d}` / `work.key: ENG` is refused, even though its `id` is exactly the pattern's shape. The configure skill documents this as a supported setup ("`id` and `external_id` may coincide when the local and remote ID schemes align"), and `work create --id` makes it easier to reach. The only remedy offered, re-keying, gives these items fresh numbers and destroys the alignment.

**Suggestion**: use one definition of "in shape" for both. Check `is_any_key_id_token` before classifying under `IdOwnership::Local`, and pick one literal-case rule for re-key and convergence. Extend `an_item_under_work_key_is_converged_exactly_when_re_key_leaves_it` with literal-text patterns, case variants and `id == external_id`.

---

### `cli/visualiser/server/src/server.rs:181-200` — The corpus-trust check is written twice: in the visualiser and in work-cli (Code Quality, Architecture)
**Severity**: major | **Confidence**: high | **Lens**: code-quality, architecture

🟡 **Code Quality**

`unconverged` copies `work-cli`'s `refuse_diverged_corpus` (`cli/work-cli/src/convergence.rs`) step by step: list the work directory, treat a listing failure as "nothing to report", run `refuse_divergence` then `refuse_malformed`, and make paths relative to the root. Only the error wrapping differs. This is review 1's "knowledge kept in two places" theme, brought back by the response.

🔵 **Architecture**

Which checks decide that work-item IDs can be trusted is domain policy, yet here it lives in the axum bootstrap. The divergence half sits under `work::rekey` even though it gates eight commands, and each command opts in separately. A third trust rule, or a change of order, has to be made in both crates, or the CLI and the visualiser silently disagree.

**Suggestion**: move the composition into `work`, e.g. `work::corpus_trust::refuse_untrusted(scheme, &files) -> Result<(), UntrustedCorpus>` with `UntrustedCorpus::{RekeyRequired, MalformedItem}`. Call it from both shells and keep only the listing and formatting in each.

---

### `CHANGELOG.md:189-201` — No upgrade path for corpora newly refused with `E_REKEY_REQUIRED` / `E_WORK_ITEM_MALFORMED`
**Severity**: major | **Confidence**: high | **Lens**: documentation

🟡 **Documentation**

This Breaking entry frames the remedy as "Run `/rekey-work-items` after changing the pattern". The users it actually hits changed the width or switched to `{tracker}` under an earlier release, or hold slugless `0400.md` files. They change nothing, upgrade, and find `create`, `list`, `resolve` and `promote` exiting 4. `### Migrations` has no entry for either refusal. Nothing says that a re-key refuses while a sync or `--push` create is unfinished, so `/sync-work-items` must run first, and the work-items docs page doesn't mention the refusal either.

**Suggestion**: add a `### Migrations` entry, e.g. "After upgrading, run `accelerator work list`. On `E_REKEY_REQUIRED`, finish any pending sync with `/sync-work-items`, then run `/rekey-work-items`. On `E_WORK_ITEM_MALFORMED`, rename the named file to `<id>-<slug>.md` or give it an `id`." Add a sentence to the work-items page's re-key paragraph too.

---

### `CHANGELOG.md:1140-1144` — Migrations entry calls re-keying optional, and nothing warns before the upgrade (Compatibility, Documentation)
**Severity**: major | **Confidence**: high | **Lens**: compatibility, documentation

🟡 **Compatibility**

Review 1's major finding, that refused patterns break every `work` command on upgrade with no prior notice, was declined by design, and the convergence gate widens it. A stable-release repository that changed only the width (documented as "absorbed transparently" until now), skipped migration 0002 with `id` fields, or holds an unpadded legacy `id` hits `E_REKEY_REQUIRED` on `list`, `resolve` and `create` as soon as it upgrades.

🔵 **Documentation**

"Edit the pattern as the reason directs, then run `/rekey-work-items` if existing items should follow it" makes the re-key optional. Under the convergence check it isn't, and the Breaking entry at the top of the file says so.

**Suggestion**: reword to "then run `/rekey-work-items`: until every item's `id` is in the edited pattern's shape, `work` commands refuse with `E_REKEY_REQUIRED`". Consider a SessionStart `config summary` line, or a no-op migration, that runs the convergence and pattern checks before the user hits them.

---

### `skills/work/create-work-item/SKILL.md:172-176` — `work resolve` now exits 4, but the resolver table has no branch for it (Usability, Documentation)
**Severity**: major | **Confidence**: high | **Lens**: usability, documentation

🟡 **Usability**

`work resolve` now refuses an off-pattern or malformed corpus with `E_REKEY_REQUIRED` / `E_WORK_ITEM_MALFORMED` at exit 4. Step 0's resolver table still stops at exits 0, 1, 2, 3 and 6, and *Identity-run refusals* is scoped to `id check`, `id next` and `create`. A topic-string run's first command meets a refusal the skill doesn't map. The nearest branch (exit 3, "treat as topic") sends the user into a full drafting conversation that then fails at Step 5.

🔵 **Documentation**

The PR description says these codes "stop the skill with their remedy", but the documented control flow has no branch for the most likely first failure after upgrade.

**Suggestion**: add "**Exit 4** — apply *Identity-run refusals*" to this table, or widen that section to include `accelerator work resolve`. Add the same branch to `update-work-item`, `review-work-item`, `list-work-items` and `sync-work-items`. Consider a guard test alongside `test_retired_id_verbs.py` that checks each skill calling `resolve` or `list` names both codes.

---

### `cli/work-cli/tests/cli_rekey_required.rs:118-147` — No test pins that `work sync` keeps running over a diverged corpus
**Severity**: major | **Confidence**: high | **Lens**: test-coverage

🟡 **Test Coverage**

`sync` is left off the `E_REKEY_REQUIRED` guard on purpose: a re-key refuses while a sync or `--push` create is unfinished, and only sync clears that state. This file tests the other two exemptions (`id canonicalise` and `rekey`) but not `sync`, and every sync fixture holds converged IDs.

**Impact**: if someone later adds `refuse_diverged_corpus` to sync (an easy mistake, since nearly every sibling calls it), no test fails, and users with an unfinished push in an off-pattern corpus are deadlocked.

**Suggestion**: add `sync_is_not_refused_over_a_diverged_corpus` using `diverged_repo()`. Assert that stderr has no `E_REKEY_REQUIRED` and that the exit code isn't 4. Better still, seed a pending-push marker and show that sync clears it and re-key then succeeds.

---

### `cli/work/src/rekey/convergence.rs:98-108` — The prescribed re-key also renames every `--project` item onto `work.key` (Correctness, Compatibility)
**Severity**: major | **Confidence**: medium | **Lens**: correctness, compatibility

🟡 **Compatibility** / 🔵 **Correctness**

Convergence accepts an item under any key (`XY-0042` beside `work.key: PP` passes), but the refusal's only remedy is `/rekey-work-items`. `numeric_mapping` keeps only items in target shape *under `work.key`* and re-renders every other sequenced item under it. So one stray off-pattern item blocks every ID command, and following the printed remedy then renames every `XY-*` item, its filename and every reference to it, or refuses with `E_REKEY_ID_COLLISION` when `PP-0042` already exists. References outside `meta/` (PR titles, tracker links) break silently.

**Suggestion**: have numeric re-key keep any item `is_converged` accepts, so it moves exactly the items the refusal counts. Failing that, say in the refusal and in the re-key preview that other-key items will also be folded, and list the off-pattern items so they can be fixed by hand.

---

### `cli/work-cli/src/resolve.rs:103-108` — Read-only `resolve` and `list` now refuse, stranding the skills that call them, including sync (Compatibility, Usability)
**Severity**: major | **Confidence**: medium | **Lens**: compatibility, usability

🟡 **Compatibility**

`work resolve` and `work list` now refuse a diverged or malformed corpus with exit 4. `review-work-item` and `update-work-item` list only exits 0/1/2/3/6, and `list-work-items` promises the CLI degrades and exits 0. `sync` is exempt so pending sync work can finish before a re-key. But `sync-work-items` finds item files only through `accelerator work resolve <id>`, and its recovery path offers `accelerator work promote <id>`, and both are refused. A user with an unfinished sync in a diverged corpus loops: re-key refuses until sync is done, and the sync skill can't reach its files.

🟡 **Usability**

Before this PR, `list` skipped unreadable files with a warning (`cli/work-cli/src/list.rs:585-589`). After a width-only pattern change, or one hand-written `0400.md`, the user can't list or look up any work item. The commands a user reaches for to see what's wrong are the ones that refuse.

**Suggestion**: keep the hard refusal for commands that write or allocate (`create`, `create-batch`, `id next`, `promote`). Let `resolve`, `list` and `id check` run, printing the same refusal line on stderr as a warning. Otherwise exempt what sync needs, and add an exit-4 branch to every calling skill.

---

### `cli/work/src/retirement.rs:493-499` — `plan_retirement` checks IDs only as path components, so a `"` in `old_id` still reaches the unescaped `aliases` write
**Severity**: minor | **Confidence**: high | **Lens**: security

🔵 **Security**

`plan_retirement` checks `old_id` and `new_id` only with `is_one_path_component`. A synced item's `id` is never checked against the ID alphabet before `retired_aliases` pushes it through `quoted_list`, which writes `format!("\"{item}\"")` with no escaping. `read_field_raw` strips only the outermost quotes. So a committed item with `id: "PP-1", "ENG-99"` and a valid `external_id`, re-keyed to `{tracker}`, gets `aliases: ["PP-1", "ENG-99"]`: an injected alias. `tracker_mapping` in `rekey/mapping.rs` applies the new alphabet check only to unsynced items, so the hardening claim ("a quote or newline cannot reach frontmatter") does not hold on a first, uninterrupted run.

**Suggestion**: refuse any ID that fails `is_in_id_alphabet` here, next to the path-component check, so every retirement (re-key, promotion, resume) goes through one gate. Optionally also make `quoted_list` escape `"` and `\`.

---

### `cli/work-cli/src/exit_codes.rs:243-253` — Exit-code contract worked out by matching the start of a flattened refusal message (Architecture, Code Quality)
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture**

`of_failure` decides between exit 4 and exit 1 by checking whether the message starts with `"E_ID_TAKEN:"`. The refusal starts as a typed `RequestedIdRefusal`, gets flattened to a `String` in `refuse_requested_id` / `claim_id`, and is then string-matched in `main.rs` and `id.rs`. `RunOutcome::Refused { message, code }` already exists for refusals that carry their own exit code.

🔵 **Code Quality**

`work id check` (`id/check.rs:59-64`) and `--dry-run --id` (`create.rs:693,711`) have the typed refusal in hand, yet stringify it and go through the same prefix test. Rewording or wrapping the message would silently move a taken ID from exit 4 to exit 1, and `eval_contract.rs` tests the same matcher, so it would not catch this.

**Suggestion**: give `RequestedIdRefusal` an exit-code mapping, or return `RunOutcome::Refused { code }` where the refusal is still typed. Keep `of_failure` only for `create_local_item`'s string path.

---

## Per-Lens Results

### Architecture

**Summary**: The review response mostly holds together. Live linkage reads through the scheme, `scheme_from_settings` is the only way to build one, `MigrationContext` exposes identity through one method, tracker key shapes are typed, and the visualiser degrades. What remains comes mostly from the new gates: the corpus-trust policy is built twice, sits under `rekey` and is opted into per command. The `E_ID_TAKEN` exit code is string-matched. The visualiser holds scheme state as two parallel `Option`s. The dry-run ordering is only half shared.

**Strengths**: linkage now reads through the `LinkageIds` port; `scheme_from_settings` is the single way to build a scheme; `IdReadings` replaces the repeated `is_tracker_owned` branching; the visualiser's fail-fast vs availability trade-off is settled; convergence and well-formedness are pure functions; `refuse_requested_id` owns the pending-then-taken order.

**Comments**:
- `cli/work-cli/src/convergence.rs:32-48` (minor, high): the corpus-trust policy is built twice, lives under `rekey` and is opted into per command.
- `cli/visualiser/server/src/server.rs:181-200` (minor, high): the visualiser repeats the CLI's two-step check in its bootstrap.
- `cli/work-cli/src/exit_codes.rs:243-253` (minor, high): the exit code is worked out by matching the message prefix.
- `cli/visualiser/server/src/config.rs:28-35` (minor, medium): two parallel `Option`s plus a free-text `problem`.
- `cli/corpus-cli/src/main.rs:101-103` (minor, medium): linkage extract fails on a scheme error for every doc type.
- `cli/work-cli/src/create.rs:669-712` (minor, medium): the dry-run still orders the corpus checks and admission separately.
- `cli/work/src/id_scheme.rs:141-176` (suggestion, medium): tracker keys stored twice, needing a second construction step.
- `cli/tracker/src/lib.rs:909-927` (suggestion, low): two matches on integration names.

**General Findings**: the test composition still copies the shipped `WorkItemIdentity` adapter; `relative_to` lives in `retirement`.

### Correctness

**Summary**: The core logic holds. The grammar splits a whole value at most one way, the taken rule, allocation and the create-lock ordering agree, and review 1's legacy-reading gaps are closed by refusing unconverged corpora. One new inconsistency matters: convergence compares literal text case by case where re-key ignores case, so some refusals can't be cleared. There are smaller gaps in what re-key rewrites, in the key-last correction, and in review targets after an identity-key handover.

**Strengths**: the single-split claim is sound for whole-value walks; review 1's findings are genuinely addressed; allocation and `refuse_if_taken` read the same holders; `refuse_requested_id` runs under the create lock; the identity-key index is deterministic and hands the key on correctly; the arithmetic is safe.

**Comments**:
- `cli/work/src/rekey/convergence.rs:26-32` (major, high): convergence is case-exact on literals while re-key ignores case.
- `cli/work/src/rekey/convergence.rs:98-103` (minor, medium): the prescribed re-key also moves other-key items.
- `cli/work/src/id_pattern.rs:511-515` (minor, high): `segments_without_key` drops the wrong literal for `{number:04d}-{key}v`.
- `cli/visualiser/server/src/indexer.rs:1039-1062` (minor, medium): a handover leaves `reviews_by_target` on the old holder.

**General Findings**: none.

### Code Quality

**Summary**: Most of review 1's code-quality findings are fixed (`IdReadings`, `KeySource`, `HeldPrefix`, the shared `refuse_requested_id`). The new code repeats review 1's main theme: the trust check is written twice, the pre-lock steps are copied between the dry-run and the real create, and small helpers are copied across the `TrackerIds`/`PatternIds` split. The rest are smaller modelling issues: types that allow invalid states, a stale doc comment, and over-width comments.

**Strengths**: `IdReadings` grouped by purpose; `SchemeError::Key` carries a typed `KeySource`; `HeldPrefix` and `Proposal::id()`; `NoWorkItemIds` is a clean null object; typed `RekeyRequired` and `MalformedItem` refusals; the duplicated `tokens_of` and `DRAFT_SUFFIX_LENGTH` are gone.

**Comments**:
- `cli/visualiser/server/src/server.rs:181-200` (major, high): the trust check is written twice.
- `cli/work-cli/src/create.rs:669-694` (minor, medium): the pre-lock steps are copied, and the scheme is read twice behind a `Once` guard.
- `cli/visualiser/server/src/config.rs:28-35` (minor, high): paired `Option`s, where `None` means opposite things.
- `cli/visualiser/server/src/compose.rs:77-81` (minor, high): an error arm that can never run, and `SchemeError::Config` degraded.
- `cli/work/src/id_scheme/tracker_ids.rs:50-53` (minor, high): `legacy_prefix` and the alphabet predicate are each defined twice.
- `cli/work/src/id_scheme.rs:143-176` (minor, medium): tracker keys stored twice, with ownership branching outside `IdReadings`.
- `cli/work/src/id_scheme/tracker_ids.rs:86-92` (suggestion, medium): a bare tuple and a hand-worked `+1` offset.
- `cli/work/src/well_formed.rs:17-31` (minor, high): `MalformedItem` allows impossible states.
- `cli/work-cli/src/id/check.rs:21-25` (minor, high): stale `RunOutcome::Refused` doc comment.
- `cli/work-cli/src/exit_codes.rs:243-253` (minor, medium): the exit code comes from the text where the type is available.
- `cli/work-adapters/src/config.rs:252-264` (minor, medium): `work.integration` read twice, and `KeyRequired` hard-codes setting names.
- `cli/visualiser/server/src/indexer.rs:471` (minor, high): renamed comments are over 80 characters.
- `cli/work-adapters/src/retirement_records.rs:96-111` (suggestion, medium): booleans instead of parsed types.

**General Findings**: the convergence names don't cover the malformed-item check.

### Test Coverage

**Summary**: The review response is well covered by tests. Nearly every behaviour change has a test that requires it, and review 1's test-coverage findings are all fixed. The gaps are at the edges of the new guards: the `sync` exemption is unpinned, the visualiser's degrade wiring is untested end to end, and a few `well_formed` branches would survive mutation.

**Strengths**: `cli_rekey_required.rs` covers every guarded command; the convergence ≡ re-key cross-check; canonicalise's `NoMatch`, `E_PATTERN` and `E_WORK_KEY_INVALID` arms are covered at the CLI; the deleted suites' coverage is re-homed; the eval guard is a pure, synthetically tested function; retirement and pending-push hardening tests.

**Comments**:
- `cli/work-cli/tests/cli_rekey_required.rs:118-147` (major, high): no test pins the `sync` exemption.
- `cli/visualiser/server/src/server.rs:82-91` (minor, high): the degrade wiring and the malformed branch are untested end to end (the agent anchored this to diff offset 8458; remapped to source lines).
- `cli/work/src/well_formed.rs:84-116` (minor, medium): the path-order sort and the case-insensitive slug match survive mutation.
- `cli/corpus-cli/src/main.rs:101-103` (minor, medium): linkage-extract refusal on a bad scheme is untested (remapped from diff offset 4365).

**General Findings**: `--project ""` → `E_WORK_KEY_INVALID` is tested only at the unit level.

### Compatibility

**Summary**: The response fixed several compatibility problems: the visualiser degrades, the API change is additive, the `.MD` entry is accurate, and `E_ID_TAKEN` moved to exit 4 before any stable release. But the new convergence gate is the main new risk. It blocks read-only verbs, refuses the documented `id == external_id` setup, and its only remedy also renames `--project` items. Skills don't handle the new exit 4, and the sync skill depends on the non-exempt `resolve`. Upgraders still get no warning beforehand.

**Strengths**: the visualiser degrades with an additive `problem`; the retired verbs are guarded by a test and never shipped stable; `bin/accelerator` pins the launcher version; the `.MD` CHANGELOG entry is correct; `E_ID_TAKEN` moved before shipping; migrations keep the frozen readings.

**Comments**:
- `cli/work/src/rekey/convergence.rs:26-32` (major, medium): `id == external_id` can never converge.
- `cli/work/src/rekey/convergence.rs:98-108` (major, medium): the remedy renames every `--project` item.
- `cli/work-cli/src/resolve.rs:103-108` (major, medium): `resolve` exits 4, and the skills including sync don't handle it.
- `CHANGELOG.md:1140-1144` (major, medium): no warning before upgrade, and re-keying called optional.
- `cli/corpus-cli/src/main.rs:101-103` (minor, medium): linkage extract fails for every doc type.
- `skills/config/configure/SKILL.md:818-821` (minor, medium): the 0002 skip guidance contradicts the refusal.

**General Findings**: review 1's 0002 allocation-jump finding no longer describes the real upgrade.

### Usability

**Summary**: Most of review 1's usability findings are fixed: other-key confirmation, key-case restoration, the output contract, a single `Error:` vocabulary, `E_VERB_RETIRED`, `E_ID_TAKEN` exiting 4, and the `--id` docs. The new friction comes from the response itself. Corpus-wide refusals block `list`, `resolve` and `id check`, the skills weren't taught the new exit-4 refusals, the malformed check reports one file per run, and the visualiser's `problem` is invisible and fixed at boot.

**Strengths**: refusals follow the code / what / remedy shape with escaped IDs; other-key confirmation; mid-argument `--id` is asked about; the deliberate `sync`/`rekey`/`id canonicalise` exemptions; one `Error:` vocabulary; the worked `--id` docs; the visualiser keeps serving.

**Comments**:
- `cli/work-cli/src/list.rs:585-589` (major, medium): read-only `list` and `resolve` refuse outright.
- `skills/work/create-work-item/SKILL.md:172-176` (major, medium): the skills have no branch for `resolve`'s exit 4.
- `cli/work/src/well_formed.rs:75-91` (minor, high): only the first malformed file is named.
- `cli/work-cli/src/exit_codes.rs:206-209` (minor, high): exit 4 no longer means "taken".
- `cli/visualiser/server/src/server.rs:55-57` (minor, high): `problem` is invisible and fixed at boot.

**General Findings**: `E_REKEY_REQUIRED` promises a re-key fixes items that re-key refuses as unclassifiable.

### Security

**Summary**: Two of review 1's security findings are fixed: the alphabet check on retirement-record IDs and `recovery_dir` confinement. The `--id` path is well guarded. One gap remains: the alphabet check sits at decode time and in two re-key branches, not at the unescaped `aliases` write, so a synced item with a crafted `id` can still inject aliases during a re-key to `{tracker}`. The new retirement refusal also prints the hostile ID raw.

**Strengths**: decoders check the ID alphabet; `is_recovery_dir` confinement with tests; the run-plan decoder parses through the grammar and confines paths; `TrackerKeys` per tracker; `problem` paths are relative and escaped; the 255-byte and width-19 caps with no regex compiled from config; refusals escape paths and IDs.

**Comments**:
- `cli/work/src/retirement.rs:493-499` (minor, high): a `"` in `old_id` reaches the unescaped `aliases` write.
- `cli/work/src/rekey/mapping.rs:281-285` (minor, high): the alphabet check covers only unsynced items.
- `cli/work/src/retirement.rs:267-271` (minor, high): the `IdNotOnePathComponent` message prints the hostile ID unescaped.

**General Findings**: validate IDs where they are written, not only where they are decoded.

### Documentation

**Summary**: Most of review 1's documentation findings are fixed: the `.MD` entry, the `--id` example, the `work id check` output table and the verb renames throughout. The gaps sit around the convergence refusals. There is no upgrade path and no `E_WORK_ITEM_MALFORMED` remedy, and passages in the configure skill and the Migrations entry contradict convergence. Skills calling `work resolve` have no branch for its exit 4.

**Strengths**: the `.MD` entry is now accurate; the `--id` example and shape rules; the `work id check --help` output table; the rename applied throughout with a guard test; the grammar limits documented; `tasks/README.md`'s guard count is updated.

**Comments**:
- `CHANGELOG.md:189-201` (major, high): no upgrade path for newly refused corpora.
- `CHANGELOG.md:243-248` (minor, high): `E_WORK_ITEM_MALFORMED` has no remedy.
- `CHANGELOG.md:202-205` (minor, high): the Breaking entry describes an exit code that never shipped.
- `CHANGELOG.md:1140-1141` (minor, high): Migrations calls the re-key optional.
- `skills/config/configure/SKILL.md:711-712` (minor, high): still says width changes are absorbed.
- `skills/config/configure/SKILL.md:811-821` (minor, medium): the 0002 skip guidance describes refused allocation.
- `skills/work/create-work-item/SKILL.md:172-176` (minor, high): the resolver table has no exit 4.
- `cli/work-cli/src/cli.rs:239-240` (minor, high): `id check` help omits case restoration.
- `docs-site/src/content/docs/guides/sync-work-items.mdx:237-240` (suggestion, medium): "every other work command" overstates the refusal.

**General Findings**: other work skills don't document the exit-4 refusals.
