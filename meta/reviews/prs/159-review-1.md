---
type: "pr-review"
id: "159-review-1"
title: "[0296] Fix sync round-trip defects with a tracker body dialect"
date: "2026-10-15T11:33:12+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:159"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "safety", "documentation", "usability"]
review_number: 1
pr_number: 159
tags: []
last_updated: "2026-10-15T11:33:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #159 - [0296] Fix sync round-trip defects with a tracker body dialect

**Verdict:** COMMENT

This is a well-structured PR: a pure `adf` crate enforced by pup, a `BodyDialect` port, a per-fact classifier that falls back to `recover` on any unknown, and recipe-1 baselines that degrade safely for older binaries. It is backed by an 81-row classify table and golden digests tied to the parser version. Eight lenses found no critical issues. Two problems are worth fixing before merge. First, the new mid-run-edit guard covers only `record-baseline`, so a push, a retitle or a pull can still absorb or overwrite a local edit made during a run. Second, the strict ADF renderer accepts some bodies that silently lose content or do not round-trip. The other findings are clarity, layering and test-gap issues.

### Cross-Cutting Themes

- **Edits made during a run escape the new guard** (flagged by: correctness, safety, code-quality, architecture). `LocalChangedSinceClassified` protects only `record_baseline`, and even there it is a no-op when classification never read the file. Push, retitle and pull each re-read the file at apply time (`apply.rs:306`, `run.rs:1126`). `run.rs` also reads files outside the `ItemSnapshot` in `keeps_local_hash`, `dossier_for` and `check_pull`. A single per-run snapshot of each file, feeding classification, the send and the baseline, would close all of these at once.
- **The strict render treats some bodies as fixed points that are not** (flagged by: correctness, safety, compatibility, test-coverage). These include a `|` inside a code span in a table cell, a hard break in a task item or heading, dropped marks and unsafe hrefs (none refused), a multi-block table cell (untested), and assembler output that breaks ADF nesting rules. Because `as_pushed` and equivalence share the same render, these losses read as `Equivalent` and nothing ever flags them.
- **Jira knowledge leaks out of the dialect** (flagged by: architecture, code-quality). `BodyComparison::same_document` assumes JSON wire documents. `run.rs` remedies name Jira and `jira show`. `migrate-cli` picks a dialect by matching on the string `"jira"`, with a silent `PlainDialect` fallback, and in doing so links all of `jira-client`. `LinearClient` re-implements `PlainDialect` instead of delegating to it.
- **Refusal diagnostics are not actionable** (flagged by: usability, documentation). Strict refusals name the node type rather than the condition. Unrenderable dossiers drop the cause. A parse failure is reported as `terminal` I/O and as `local-unreadable`. A refused send says `pull failed`.
- **Preview and apply drift apart** (flagged by: code-quality, correctness). `check_send` and `send_outcome` are two copies of one gate, and they already disagree on an unreadable file.

### Tradeoff Analysis

- **Safety vs usability in the recipe-1 migration**: the safety lens wants a legacy item whose two sides differ in meaning to be refused or raised as a conflict rather than recorded as `synced`. That would stop known damage (0161, 0203, 0276, 0293) from vanishing into the baseline. Usability and the PR's own goal favour a quiet one-time migration. I'd recommend at least surfacing those items once, either as a conflict or a summary row, rather than silently baselining them.
- **Escaping fidelity vs readability**: escaping every `_ * [ ~` makes the Markdown round-trip safely, but it puts backslashes into `jira show` output and into pulled files (`snake\_case`). Keeping the broad escaping on the sync path and offering a lighter render for `jira show` would give both lenses what they want.

### Strengths

- ✅ The `adf` crate is a pure codec enforced by the pup rules `adf_is_a_pure_codec` and "no `adf` in work-adapters". `pulldown-cmark` sits behind two owned wrappers, and cargo-deny confines it to them.
- ✅ `classify` is a functional core over lazy `ItemDigests`/`ItemComparison` ports, with explicit `Change::Unknown` routed to `recover`, so a missing hash can never trigger a silent push or pull.
- ✅ `CanonicalBody` is the single source of both the recipe-1 hash and equivalence, with a length-prefixed, injective digest input.
- ✅ The baseline `Entry` is `Legacy | Split | Unsupported`. Unknown-recipe entries survive writes byte for byte, and recipe-1 entries keep the legacy keys so older binaries still read them.
- ✅ `golden_digests.rs` fails if the locked `pulldown-cmark` version changes without a `DigestRecipe::CURRENT` bump.
- ✅ The send refusals (residue, unfetched remote, unrenderable remote) and fail-closed pulls (70/75) close the empty-body and raw-ADF paths. `Retitle` sends only the title, and a contract property pins that.
- ✅ The classify table grows from 17 to 81 rows, with a fake that panics on facts a row did not state. Each bug fix has a regression test at the right level, and retiring `bash-parity` means the real-repository suites run under plain `cargo test`.
- ✅ Every exit-code, report-row and detail change is in the CHANGELOG with a "scripts need updating" note. The SKILL.md detail/remedy table matches the `FailureClass` mapping, and dossier `reason:` lines are read only from the header region.

### General Findings

- 🔵 **Correctness**: the `frontmatter_edit_stands` term in `reconcile_bodies` never changes the result on any reachable path. Confirm the intended case is decided elsewhere, by Pull preserving the local frontmatter.
- 🔵 **Safety**: a send does not re-check the remote `updated` stamp, so a teammate's edit to the issue during a long run is overwritten and then baselined. This predates the PR, but the push path is rewritten here.
- 🔵 **Usability**: the one-time `last-sync.json` migration produces no report row, so an operator sees "everything synced" followed by a diff of hundreds of lines. Consider a `#\tsummary\trebaselined\t<n>` row.
- 🔵 **Usability**: `unsupported-recipe` exits `4` ("awaiting a human") with nothing on stderr. Outside the skill, the only fix is to upgrade, and nothing says so.
- 🔵 **Usability**: the reconcile-pull refusal that remains until 0300 now exits `71`, which SKILL.md describes as "whether the write landed is unknown". Here nothing was written.
- 🔵 **Compatibility**: a mixed-version team will churn the committed `last-sync.json` back and forth. Add a migration note recommending a team-wide upgrade.
- 🔵 **Architecture**: `run.rs` has taken on payload composition, parsing, refusals, dossier assembly and remedy copy. Extract the dossier work into its own module.
- 🔵 **Architecture**: one `BodyParser` is threaded by hand through five port structs. Inject a ready-built `BaselineDigester`/`BodyComparison`, or a `BodyRules` bundle, instead.
- 🔵 **Code Quality**: `ItemSnapshot`'s `ItemComparison` methods swallow the same read errors that its `ItemDigests` methods propagate.
- 🔵 **Code Quality**: `PlanInput` takes one snapshot as two trait objects, `digests` and `comparison`. No caller ever supplies different implementations for them.
- 🔵 **Test Coverage**: `projection_corpus.rs` and the sync-baseline corpus lost the "every case directory has a test arm" check the bash-parity guard used to provide.
- 🔵 **Documentation**: the Errors section of `document_to_markdown_strict` names only headings above level six. The CHANGELOG also lists a hard break in a table cell, a backtick in code text, and a fence line in a code block.
- 🔵 **Documentation**: the unparseable-frontmatter change appears under both Breaking and Fixed.

### Additional Findings

- 🟡 `cli/migrate-cli/src/sync_baselines.rs:23` — dialect chosen by matching on the string `"jira"` with a silent `PlainDialect` fallback, which links all of `jira-client`, and has no test (architecture, code-quality, test-coverage)
- 🟡 `skills/work/sync-work-items/SKILL.md:225` — `unrenderable` parenthetical contradicts itself, and the exit-code run-on paragraph needs a table (documentation, usability)
- 🔵 `cli/adf/src/render.rs:184` — hard break in a task item or heading is not refused but does not round-trip (correctness)
- 🔵 `cli/adf/src/render.rs:296` — strict refusal of a multi-block table cell is untested (test-coverage)
- 🔵 `cli/work/src/sync/classify.rs:505` — legacy→recipe-1 migration records items whose sides differ in meaning as synced (safety)
- 🔵 `cli/work/src/sync/classify.rs:482` — verbatim-remote residue is baselined and never reported again (correctness)
- 🔵 `cli/work/src/sync/classify.rs:387` — a remote-only title change is absorbed into the baseline and never reaches the local file (0290) (safety)
- 🔵 `cli/work-adapters/src/sync/baseline.rs:355` — `Entry::Split` forgets its recipe and is always rendered as `CURRENT` (architecture)
- 🔵 `cli/work-adapters/src/sync/baseline.rs:306` — unknown-recipe entries ignore their legacy keys, so this binary cannot degrade the way it lets older binaries degrade (compatibility)
- 🔵 `cli/work-adapters/src/sync/body_comparison.rs:170` — JSON wire-document equality is hard-coded outside the dialect (architecture, code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:477` — tracker-agnostic run code names Jira in its remedies (architecture, code-quality)
- 🔵 `cli/work-adapters/src/sync/run.rs:421` — unparseable frontmatter is reported as `terminal` I/O (usability, code-quality `run.rs:403`)
- 🔵 `cli/work-adapters/src/sync/run.rs:467` — the `local-unreadable` dossier says "could not be read" when the frontmatter failed to parse (usability)
- 🔵 `cli/work-adapters/src/sync/run.rs:480` — the residue and legacy remedies ask for a manual revert, sync, reapply sequence (usability)
- 🔵 `cli/work-adapters/src/sync/run.rs:847` — `Payload` re-encodes the action as two `Option`s (code-quality)
- 🔵 `cli/work-adapters/src/sync/apply.rs:105` — a refused send reports `pull failed` (usability)
- 🔵 `cli/work-adapters/src/sync/apply.rs:112` — the pull-residue remedy uses a placeholder `<key>` command (usability)
- 🔵 `cli/work-adapters/src/sync/send_refusal.rs:67` — send-refusal policy sits in the adapter crate next to the domain's `push_precondition` (architecture)
- 🔵 `cli/work-adapters/src/sync/send_refusal.rs:12` — `EntryShape` is a third model of the recipe, and an unsupported entry reads as `Legacy` (code-quality)
- 🔵 `cli/work-adapters/src/sync/send_refusal.rs:83` — a split overwrite of an unrenderable remote keeps no copy of the ADF it replaces (safety)
- 🔵 `cli/work-adapters/src/sync/pending_push.rs:306` — the split-fingerprint keys and assembly are duplicated across two codecs (code-quality)
- 🔵 `cli/work-adapters/src/sync/pending_push.rs:29` — promotion records are always written as schema 3 (compatibility)
- 🔵 `cli/work-adapters/src/promotion.rs:820` — `finish_promotion` bypasses the file port and maps a failed `show` to `Synced` (architecture)
- 🔵 `cli/work-adapters/src/sync/realign.rs:83` — realign's dialect wiring is never tested on Jira (test-coverage)
- 🔵 `cli/work/src/sync/classify.rs:564` — a digest computation is used to ask whether the remote was fetched (code-quality)
- 🔵 `cli/work/src/sync/classify.rs:413` — the compound boolean conflict rule is hard to verify (code-quality)
- 🔵 `cli/work/src/sync/decide.rs:121` — one concept goes by `SendFields`, `Retitle` and `Payload{body: None}` (code-quality)
- 🔵 `cli/work-cli/src/sync.rs:307` — `action_keyword` disagrees with `Action`'s `Display`, and a retitle cannot be told from a push in the report (code-quality, usability)
- 🔵 `cli/work-cli/src/sync.rs:4415` — the Jira arm of the pull-create test asserts that raw ADF is written (test-coverage)
- 🔵 `cli/adf/src/syntax.rs:8` — escaping every `_ * [ ~` adds backslashes to `jira show` output and to pulled files (usability)
- 🔵 `cli/visualiser/server/src/watcher.rs:222` — the before-snapshot is taken outside `rescan_lock` (architecture); the detached rescan and `rescan_changes` lack direct tests (test-coverage, lines 176, 260, 844)
- 🔵 `cli/document/src/markdown.rs:67` — two `pulldown-cmark` wrappers parse different dialects (architecture)
- 🔵 `cli/jira-client/src/lib.rs:52` — `jira-client` still re-exports the `adf` API, which bypasses the pup rule (architecture)
- 🔵 `CHANGELOG.md:326` — `pull-residue` exit 75 sits under Changed, not Breaking (documentation); `CHANGELOG.md:321` — the `work diff` output change likewise (compatibility)
- 🔵 `CHANGELOG.md:642` — the recipe-1 migration entry leaves out downgrade behaviour (documentation)
- 🔵 Smaller: `tracker/src/lib.rs:627` (`rejects` docstring), `tracker/src/lib.rs:727` (`retitle` vs synced fields), `linear-client/src/client.rs:920` (re-implements `PlainDialect`), `adf/src/render.rs:60` (comments reference retired jq), `adf/src/assemble.rs:25` (over 80 columns), `golden_digests.rs:302` (no recipe-gap case), `sync_classify.rs:327` (call-flag coupling), `SKILL.md:206` (`unsupported-recipe` placement), `work-cli/src/sync.rs:422` (`failed\tsynced`)

## Inline Comments

### `cli/adf/src/render.rs:421-425` — Code span holding `|` in a table cell is neither escaped nor refused
**Severity**: major | **Confidence**: high | **Lens**: correctness

🟡 **Correctness**

The code branch of `Marks::wrap` produces ``format!("`{text}`")`` and skips the `also` set, which is where `Surroundings::TableCell` adds `|`. In strict mode the only check on code text is the backtick test at line 322. A Jira table cell whose code mark holds `a|b` therefore renders as ``| `a|b` |``. Under GFM that row has one cell too many, the header stops matching the delimiter row, and the whole table reads back as paragraph text.

**Impact**: `JiraBodyDialect::local_body` returns `Ok`, so a pull writes the broken table into the work item. Because `as_pushed` reproduces the same Markdown, equivalence then calls the two sides equivalent, and the next push sends a paragraph of pipes to Jira. Pipe tables are new in this PR, so this failure mode is new too: shell pipelines, type unions and regexes in table cells all hit it.

**Suggestion**: In strict mode, refuse a code run (and a link href) that contains `|` when `surroundings == Surroundings::TableCell`, the same way the backtick case is refused. Add a fixed-point fixture for that cell shape.

---

### `cli/work-adapters/src/sync/item_snapshot.rs:88-99` — `LocalChangedSinceClassified` guard does nothing when classification never read the file
**Severity**: major | **Confidence**: medium | **Lens**: correctness

🟡 **Correctness**

`record_baseline` refuses when the file's current legacy digest differs from `request.fingerprints.legacy.local`. Those fingerprints come from `recorded()`, which reads the file lazily. On a recipe-1 entry, classification can skip the read entirely: the mtime pre-filter proves the local side `Unchanged`, the remote stamp moves with only `remote.fields` changed (a remote retitle), and `reconcile_fields` returns `Synced(RecordOnly)` without consulting `ItemComparison`. `recorded()` then reads the file at apply time, and the guard compares that content with itself, so it always passes.

**Impact**: An edit made between planning and this item's apply is recorded as synced. The next run sees local equal to baseline, never pushes the edit, and a later remote change pulls over it. This is exactly the absorption the guard exists to prevent.

**Suggestion**: Make the guard independent of when the content is read. Either capture mtime/content during classification for every item that may be recorded, or pass the classification-time mtime into `RecordRequest` and refuse when the current mtime differs.

---

### `cli/work-adapters/src/sync/apply.rs:306-316` — Push/retitle baseline absorbs local edits made after classification
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`record_read_back` re-reads the local file after `update`/`retitle` and `show`, and records that content as the synced baseline. It never checks the content against what was classified or sent. With `Retitle` and `Push(FieldDelivery::Withhold)`, only part of the item is sent. If the file changes before this read, the unsent part is recorded as synced: a retitle absorbs a body edit, and a withheld push absorbs a title edit.

**Impact**: An edit is silently dropped. The item reports `synced` while the tracker lacks the change, and the split recipe never sees it as a local change again.

**Suggestion**: Record from the exact content `send_outcome` read and sent, or reuse `record_baseline`'s guard and refuse with `LocalChangedSinceClassified` when the re-read differs. This shares a root cause with the `item_snapshot.rs` and `run.rs` pull findings: one per-run snapshot of each file should feed classification, the send and the baseline.

---

### `cli/work-adapters/src/sync/run.rs:1126-1129` — Pull overwrites a local body edited after classification
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`pulled_file` reads the local file fresh at apply time and builds `local.with_body(&remote_body)`, which `ItemApplier::pull` writes atomically. Nothing checks that the local body still matches the snapshot that classified the item as `remotely-modified` with an unchanged local side.

**Impact**: A body edit made between planning and this item's turn is replaced wholesale by the remote body, and the baseline then records the overwritten file as synced. No conflict and no report are raised, and the edit is recoverable only from VCS history if it was committed.

**Suggestion**: Before writing, compare the fresh content's legacy digest with the classified snapshot's `legacy_local`, as `record_baseline` does. On a mismatch, fail with `LocalChangedSinceClassified` (retryable) so the next run sees a conflict.

---

### `cli/adf/src/render.rs:363-400` — Strict render drops unsupported marks and unsafe link targets, so the unrenderable-push guard never fires for them
**Severity**: major | **Confidence**: medium | **Lens**: safety

🟡 **Safety**

`Marks::of` has no refusal path. The strict render, which `JiraBodyDialect::local_body` uses for every pull, digest and equivalence check, refuses unsupported nodes but silently drops two things:

- `underline`, `textColor`, `subsup`, `backgroundColor`, and any other mark without a Markdown form.
- Any link whose `href` fails `href_is_safe`, such as `slack://` or `ftp://`.

`a_strict_render_drops_underline` pins this behaviour.

**Impact**: The rendered remote hides the loss. Equivalence reports `Equivalent`, `send_refusal` never sees `RemoteUnrenderable`, and the next local body edit pushes Markdown that strips that formatting and those link targets from Jira. That is the loss the unrenderable refusal exists to prevent.

**Suggestion**: In `NoMarkdownForm::Refusal` mode, call `admit_unrepresentable` for an unsupported mark or a dropped href. Keep the silent drop for the display-only `document_to_markdown` path.

---

### `cli/adf/src/assemble.rs:222-237` — Assembler emits ADF nesting Jira's schema rejects, replacing a local exit 41 with a remote HTTP 400
**Severity**: major | **Confidence**: medium | **Lens**: compatibility

🟡 **Compatibility**

`Token::BlockQuote` and list items are assembled by a recursive `blocks()` call over whatever the tokeniser read, and the tokeniser accepts headings, pipe tables, rules, task lists and nested `>` inside both. ADF forbids those children:

- `blockquote` holds only paragraph, lists, codeBlock and media.
- `listItem` holds only paragraph, lists, codeBlock and media.
- `table` and `rule` are top-level blocks.

**Impact**: Inputs such as `> ## Note`, a pipe table inside `>`, `> > nested`, or a heading indented under a list item now reach Jira and fail with 400 `INVALID_INPUT`, where the old converter refused them locally with exit 41. `rejects` returns `false` for these bodies, so `work sync --preview` reports a valid payload for a push that will fail, and `as_pushed` predicts a read-back Jira will never store.

**Suggestion**: Enforce the ADF content model during assembly, either by refusing or by flattening (for example, a heading inside a quote becomes a strong paragraph). Make `rejects` reflect the result, and add extension fixtures for each disallowed nesting.

---

### `cli/adf/src/render.rs:199-201` — Strict-render refusals name node types that do have a Markdown form
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

Three refusals pass `admit_unrepresentable` only the node type: a code block holding a fence line (here), code text holding a backtick (line 322), and a hard break in a table cell (line 336). The resulting `work sync` stderr reads, for example, `E_ADF_UNREPRESENTABLE: a codeBlock has no Markdown form`.

**Impact**: Operators know code blocks, inline code and line breaks all work, so the message looks wrong, and it does not say which construct to change. The documented remedy, 'edit the description in Jira until it renders', cannot be followed.

**Suggestion**: Carry a reason in `AdfError::Unrepresentable` that names the condition, for example `a code block containing a line starting with three backticks`, `inline code containing a backtick`, or `a line break inside a table cell`.

---

### `cli/work-adapters/src/sync/run.rs:763` — Unrenderable dossiers drop the cause, so 'edit until it renders' gives no clue what to edit
**Severity**: major | **Confidence**: high | **Lens**: usability

🟡 **Usability**

`comparison.local_body(remote_projection).map_or_else(|_| ...)` discards the `Unrenderable` cause, for example `a panel has no Markdown form`. The `remote-unrenderable`, `legacy-unrenderable` and `local-pull-residue` dossiers then tell the user to edit the description in Jira without naming the offending node or mark, and the skill relays that remedy verbatim.

**Impact**: On a long Jira description the operator has to guess and re-run until the conflict clears.

**Suggestion**: Keep the cause in `ConflictDossier` and print it in the header, for example `cause: a panel has no Markdown form`. Combined with the `render.rs:199` finding, this makes the remedy actionable.

---

### `cli/work-adapters/src/sync/run.rs:887` — Preview `check_send` duplicates apply `send_outcome` and already diverges on an unreadable file
**Severity**: major | **Confidence**: high | **Lens**: code-quality, correctness

🟡 **Code Quality**: `check_send` (preview) and `send_outcome` (apply) each repeat the same steps: read the file, `LocalWorkItem::parse`, `send_refusal(...)`, `Payload::of(...)`. Two hand-maintained copies of one gate will keep drifting.

🔵 **Correctness**: they have already drifted. Here `std::fs::read_to_string(&item.path).ok()?` turns an unreadable file into `None`, so `validate_pushes` reports `NotApplied` with no failure. Apply maps the same error through `read_local` to `ApplyError::Io` (exit 71). The parse-failure branch was fixed to return `Some(Err(...))`, but this one was not. That contradicts the PR's claim that 'Preview reports the failure a planned pull or send would meet'.

**Suggestion**: Extract one `prepare_send(item, action, ...) -> Result<PreparedSend, ApplyError>`. Preview validates its result and apply sends it, so both report the same failure from the same code.

---

### `cli/work-adapters/src/sync/run.rs:1221-1230` — Data clump threaded through the run loop forces a `too_many_arguments` allow
**Severity**: major | **Confidence**: high | **Lens**: code-quality

🟡 **Code Quality**

`facts: &GatheredFacts`, `comparison: &BodyComparison`, `shapes: &EntryShapes` and `snapshots` travel together through about eight functions: `apply_planned_action`, `send_outcome`, `validate_pushes`, `check_send`, `keeps_local_hash`, `build_dossiers`, `dossier_for` and `readable_dossier`. Keeping them separate takes `#[allow(clippy::too_many_arguments)]` here, and `readable_dossier` packs a tuple parameter to stay under the limit. Separately, `ItemIndex` pairs items with `snapshots` by position (line 649), which silently mismatches if either collection is reordered.

**Impact**: Every new per-run fact means editing all of these signatures, and the unnamed clump hides that these values are one concept: what this run knows about each item.

**Suggestion**: Introduce a run-scoped value, for example `RunKnowledge { facts, comparison, shapes, snapshots }`, with `projection(id)`, `shape(id)` and `snapshot(id)` keyed by id. That also gives the mid-run-edit fixes a single snapshot to check against.

---

## Per-Lens Results

### Architecture

**Summary**: The PR is structurally sound. The pure `adf` crate is enforced by pup. `pulldown-cmark` is wrapped behind `document::markdown` and `corpus::body`, with a `BodyParser` port. The `BodyDialect` port and a pure classifier are good layering. The concerns are about placement. Choosing a dialect by integration name happens in `migrate-cli` and pulls the Jira HTTP client into the migration binary. Jira wire-format knowledge, namely JSON equality and remedy text that names Jira, has leaked into tracker-agnostic code. Send-refusal policy has grown in `work-adapters` beside the domain's `push_precondition`.

**Strengths**: the `adf` extraction and the `adf_is_a_pure_codec` rule; the two-layer `pulldown-cmark` wrapping with a cargo-deny wrapper list; `BodyDialect` as a separate supertrait with no default for `is_wire_document`; `classify` as a functional core that returns a richer `Classification`; the explicit `Legacy | Split | Unsupported` entry enum with byte-for-byte preservation; and `golden_digests.rs` tying the parser pin to a recipe bump.

**Comments**:
- `cli/migrate-cli/src/sync_baselines.rs:23-28` — major/medium — the migration binary picks its dialect by string and links all of `jira-client`. Use an exhaustive `Integration` match and a transport-free home for `JiraBodyDialect`.
- `cli/work-adapters/src/sync/body_comparison.rs:170-178` — minor/high — JSON wire-document equality is hard-coded outside the dialect. Add `BodyDialect::same_wire_document`.
- `cli/work-adapters/src/sync/run.rs:477-483` — minor/high — tracker-agnostic run code names Jira in its remedies.
- `cli/work-adapters/src/promotion.rs:820-832` — minor/medium — `finish_promotion` bypasses the injected reader, adds a tracker read, and maps a failed show to `Synced`.
- `cli/tracker/src/lib.rs:727-728` — suggestion/medium — the port says `retitle` where the domain says synced fields.
- `cli/work-adapters/src/sync/baseline.rs:355-356` — minor/medium — `Entry::Split` forgets its recipe and is always rendered as `CURRENT`.
- `cli/work-adapters/src/sync/send_refusal.rs:67-85` — minor/medium — send-refusal policy sits in the adapter beside the domain `push_precondition`.
- `cli/document/src/markdown.rs:67-71` — minor/medium — two `pulldown-cmark` wrappers configure different dialects.
- `cli/jira-client/src/lib.rs:52-54` — suggestion/medium — the `adf` API is still re-exported, bypassing the pup rule.
- `cli/visualiser/server/src/watcher.rs:222-223` — minor/medium — the rescan's before-snapshot is taken outside `rescan_lock`.

**General Findings**:
- minor — `run.rs` has grown several responsibilities and re-reads files outside the classification snapshot.
- suggestion — the single `BodyParser` implementation is threaded by hand through five port structs.

### Correctness

**Summary**: The classifier logic is mostly sound. Per-fact change detection, the handling of split, legacy and unsupported entries, the decide table and the exit-code mapping are all internally consistent. There are two substantive defects. A code span containing `|` in a table cell renders unescaped and is not refused. The `LocalChangedSinceClassified` guard does nothing when classification never read the file. Smaller gaps: preview parity, strict-render fixed points for hard breaks, and residue baked into recipe-1 baselines.

**Strengths**: positional title stripping in `split_projected`; per-fact reasoning with `Unknown` routed to `recover`; one `CanonicalBody` behind both the hash and equivalence; `Unsupported` entries never read or blanked; the two-tier bulk read skips `show` only when both sides are proven unchanged; `definitively_reconciled` keeps the watermark for a deferred `Noop`; a total `FailureClass` match.

**Comments**:
- `cli/adf/src/render.rs:421-425` — major/high — a code span holding `|` in a table cell is neither escaped nor refused.
- `cli/work-adapters/src/sync/item_snapshot.rs:88-99` — major/medium — the guard does nothing when classification never read the file.
- `cli/adf/src/render.rs:184-186` — minor/medium — a hard break in a task item or heading is not refused, and it does not read back as itself.
- `cli/work-adapters/src/sync/run.rs:887` — minor/high — preview silently skips an unreadable file that apply fails with exit 71.
- `cli/work/src/sync/classify.rs:482-496` — minor/low — verbatim-remote residue is recorded as synced and never reported again.

**General Findings**:
- suggestion — `frontmatter_edit_stands` never decides the result on any reachable path.

### Code Quality

**Summary**: The new domain types are well named: `BodyDialect`, `BodyComparison`, `CanonicalBody`, `Classification`, and `Entry` with `SyncStamps`. The maintainability cost is concentrated in `run.rs`. A data clump threaded through about eight functions needs a `too_many_arguments` allow, and the preview and apply send paths duplicate each other and have already drifted. Smaller issues: the recipe kind is modelled three times, `Action` has two keyword mappings that disagree, and a digest is used as a presence check.

**Strengths**: `BodyComparison` as the single owner of body relations; a small, length-prefixed `CanonicalBody`/`DigestInput`; explicit `Change` values and a `Classification` enum; `Entry` as an enum that owns its codec conventions; `ItemSnapshot` built on `OnceCell` without memoising failures; a shared `syntax` module for render and tokenise; `ApplyError::class()` made total.

**Comments**:
- `cli/work-adapters/src/sync/run.rs:1221-1230` — major/high — a data clump forces `too_many_arguments`.
- `cli/work-adapters/src/sync/run.rs:887` — major/high — `check_send` duplicates `send_outcome` and has diverged on read failure.
- `cli/work/src/sync/classify.rs:564` — minor/high — a digest computation is used as the "was the remote fetched?" check.
- `cli/work/src/sync/classify.rs:413-422` — minor/medium — the compound boolean conflict rule is hard to verify.
- `cli/work/src/sync/classify.rs:307` — suggestion/high — the `recorded_anything` match and its unreachable fallback are duplicated.
- `cli/work-cli/src/sync.rs:307-309` — minor/high — a second `Action` keyword mapping disagrees with `Display`.
- `cli/work/src/sync/decide.rs:121` — suggestion/medium — the term drifts from `SendFields` to `Retitle` to `Payload{body: None}`.
- `cli/work-adapters/src/sync/run.rs:847-850` — minor/high — `Payload` re-encodes the action as two `Option`s.
- `cli/work-adapters/src/sync/run.rs:649` — minor/medium — `ItemIndex` pairs items with snapshots by position.
- `cli/work-adapters/src/sync/body_comparison.rs:170-178` — minor/medium — the agnostic comparison hard-codes JSON.
- `cli/migrate-cli/src/sync_baselines.rs:23-28` — minor/medium — a raw string selects the integration, with a silent `Plain` fallback.
- `cli/work-adapters/src/sync/pending_push.rs:306-322` — minor/high — split-fingerprint keys and assembly are duplicated across two codecs.
- `cli/work-adapters/src/sync/send_refusal.rs:12-28` — suggestion/medium — `EntryShape` is a third model of the recipe.
- `cli/work-adapters/src/sync/run.rs:403` — suggestion/medium — a stringly-typed parse error is reported as `Io`.
- `cli/linear-client/src/client.rs:920-924` — suggestion/medium — `LinearClient` re-implements `PlainDialect`.
- `cli/adf/src/render.rs:60-62` — minor/high — comments reference the retired jq and regex reference implementation or restate the code.
- `cli/adf/src/assemble.rs:25` — suggestion/medium — lines inside `json!` exceed 80 columns (also at line 252).

**General Findings**:
- minor — `ItemComparison` methods swallow read errors that the `ItemDigests` methods propagate.
- suggestion — one snapshot is passed as two trait objects.
- suggestion — remedy text names the tracker inside tracker-agnostic run code.

### Test Coverage

**Summary**: Coverage is broad and behavioural. It includes the 81-row classify table, named sync scenarios, golden digests with a parser guard, and real-client pull entry points. Retiring `bash-parity` adds coverage, because the real-repository suites now run under plain `cargo test`. The gaps are narrower. Realign's dialect wiring is untested on Jira. One strict refusal is untested. The watcher's two new mechanisms have weak regression tests. One Jira arm exercises no Jira behaviour.

**Strengths**: the 81-row table with a fake that panics; the real-repository suites running unconditionally; golden digests plus a table-tested guard; a focused unit test for each `send_refusal` branch; bug-fix regression tests at the right level; a scripted `DirectoryWatcher`; an e2e test made deterministic through PATCH.

**Comments**:
- `cli/work-adapters/src/sync/realign.rs:83` — minor/medium — realign's dialect wiring is never tested on Jira.
- `cli/migrate-cli/src/sync_baselines.rs:23-28` — minor/medium — the `jira → JiraBodyDialect` mapping is untested.
- `cli/adf/src/render.rs:296` — minor/high — the strict refusal of a multi-block table cell is untested.
- `cli/visualiser/server/src/watcher.rs:176-188` — minor/medium — nothing pins why the rescan is detached.
- `cli/visualiser/server/src/watcher.rs:844` — minor/medium — the absorbed-change test relies on a 300 ms sleep.
- `cli/visualiser/server/src/watcher.rs:260` — minor/medium — `rescan_changes` semantics changed without direct tests.
- `cli/work-cli/src/sync.rs:4415-4439` — minor/high — the Jira arm asserts that raw ADF JSON is written.
- `cli/work-adapters/tests/golden_digests.rs:302` — suggestion/high — the recipe guard has no case for a gap in recipe numbers.
- `cli/work/tests/sync_classify.rs:327` — suggestion/low — the `expect_comparison_called` flag couples every row to evaluation order.

**General Findings**:
- minor — retiring `bash_parity_baseline.rs` removed the "case directory has a test arm" check for `projection_corpus.rs` and the sync-baseline corpus.

### Compatibility

**Summary**: Persisted state is handled carefully. Recipe-1 entries keep the legacy keys. Legacy entries render byte-identically. Unknown recipes survive writes. Schema-2 promotion records still parse. The CHANGELOG lists every exit code and report row. The main gap is the widened Markdown→ADF assembler: it now emits nesting the ADF schema forbids, so what used to be a local exit 41 becomes a remote 400.

**Strengths**: legacy keys kept, with a downgrade-path test; `Entry::Unsupported` round-trips byte for byte; schema 2 and 3 both read; `pulldown-cmark` pinned exactly and confined, with a digest guard; partial updates that omit an unchanged title; Breaking-section discipline, with the skill updated in the same change. The rustls pin is already on the base branch.

**Comments**:
- `cli/adf/src/assemble.rs:222-237` — major/medium — the assembler emits ADF nesting that Jira rejects.
- `cli/work-adapters/src/sync/baseline.rs:306-313` — minor/medium — an unknown-recipe entry discards its legacy keys.
- `cli/work-adapters/src/sync/pending_push.rs:29-33` — suggestion/low — promotion records are always written as schema 3.
- `CHANGELOG.md:321-325` — suggestion/low — the `work diff` output change is filed under Changed, not Breaking.

**General Findings**:
- minor — mixed plugin versions will churn the committed `last-sync.json`.

### Safety

**Summary**: The PR substantially improves safety. Unparseable frontmatter no longer blanks an issue. Residue, unfetched remotes and unrenderable remotes are all refused. Local I/O fails closed, and newer baselines are preserved. Gaps remain. The mid-run-edit check protects only `record-baseline`, so push, retitle and pull can absorb or overwrite edits. The strict renderer drops marks and unsafe hrefs without refusing them. The recipe-1 migration baselines items whose two sides differ.

**Strengths**: parse failures stop both send and write; `send_refusal` covers residue, unfetched and unrenderable remotes; `record_baseline` re-hashes before recording; unknown-recipe entries are never blanked; legacy keys are kept for downgrade; `Retitle` sends only the title; `recover` never overwrites silently; pulls fail closed; resolved overwrites count against `max_pushes`.

**Comments**:
- `cli/work-adapters/src/sync/apply.rs:306-316` — major/medium — push and retitle baselines absorb mid-run edits.
- `cli/work-adapters/src/sync/run.rs:1126-1129` — major/medium — a pull overwrites a local body edited after classification.
- `cli/adf/src/render.rs:363-400` — major/medium — the strict render drops unsupported marks and link targets.
- `cli/work/src/sync/classify.rs:505-510` — minor/medium — migration records divergent items as synced.
- `cli/work/src/sync/classify.rs:387-392` — minor/high — a remote-only title change is absorbed into the baseline.
- `cli/work-adapters/src/sync/send_refusal.rs:83-90` — suggestion/medium — a split overwrite over unrenderable content keeps no copy.

**General Findings**:
- minor — sends do not re-check the remote stamp, so concurrent tracker edits can be overwritten.

### Documentation

**Summary**: The docs mostly keep pace with the code. Report rows, `75` details and exit codes match what `work-cli` prints. The main problems are in the SKILL.md exit-code paragraph: a new `unrenderable` clause contradicts itself, and the paragraph has grown hard to read. The CHANGELOG files `pull-residue` under Changed rather than Breaking, and it leaves out the downgrade behaviour.

**Strengths**: the detail/remedy table matches `FailureClass`; CHANGELOG entries carry "scripts need updating" notes; Step 3 dossier handling covers each `reason:`; doc comments give rationale only where needed; the `tasks/README.md` public-API note was updated.

**Comments**:
- `skills/work/sync-work-items/SKILL.md:225-229` — major/medium — the `unrenderable` parenthetical contradicts itself.
- `skills/work/sync-work-items/SKILL.md:206-208` — minor/high — the `unsupported-recipe` text is wedged into the run-on sentence.
- `CHANGELOG.md:326-329` — minor/medium — `pull-residue` exit 75 is under Changed, not Breaking.
- `CHANGELOG.md:642-648` — minor/medium — the recipe-1 migration entry leaves out downgrade behaviour.
- `cli/tracker/src/lib.rs:627-628` — suggestion/medium — the second clause of the `rejects` doc comment is hard to follow.

**General Findings**:
- suggestion — the `document_to_markdown_strict` rustdoc names only one contextual refusal.
- suggestion — the unparseable-frontmatter change appears under both Breaking and Fixed.

### Usability

**Summary**: Operators and the skill agent get much more signal: detail tokens, a `reason:` line and a remedy in each dossier, and a reduced `[local/skip]` prompt. Several messages still mislead. Strict refusals name node types that do have a Markdown form. Unrenderable dossiers drop the cause. Parse failures read as terminal I/O and as unreadable files.

**Strengths**: distinct report tokens per failure class; machine-readable `reason:` lines; a reduced `[local/skip]` prompt for remote-unrenderable conflicts; preview reports refusals ahead of an applied run; a `title:` diff header, with notation-only sections omitted; a remedy table.

**Comments**:
- `cli/adf/src/render.rs:199-201` — major/high — refusals name node types that do have a Markdown form.
- `cli/work-adapters/src/sync/run.rs:763` — major/high — unrenderable dossiers drop the cause.
- `cli/work-adapters/src/sync/run.rs:421-427` — minor/high — unparseable frontmatter is reported as terminal I/O.
- `cli/work-adapters/src/sync/run.rs:467-471` — minor/high — the `local-unreadable` dossier says "could not be read" when the frontmatter failed to parse.
- `cli/work-adapters/src/sync/apply.rs:105-108` — minor/high — `RemoteBodyUnavailable` says "pull failed" when a push was refused.
- `cli/work-adapters/src/sync/apply.rs:112-118` — minor/medium — the pull-residue remedy uses a placeholder command.
- `cli/work-adapters/src/sync/run.rs:480-501` — minor/medium — the remedies ask for a manual revert, sync, reapply sequence.
- `cli/work-cli/src/sync.rs:307-309` — minor/medium — retitles and overwrites both report as `push`.
- `cli/work-cli/src/sync.rs:422-424` — suggestion/low — `failed\tsynced` is a surprising row.
- `cli/adf/src/syntax.rs:8` — minor/medium — broad escaping adds backslashes to `jira show` output and to pulled files.
- `skills/work/sync-work-items/SKILL.md:226-230` — minor/medium — the exit-code guidance is wedged into a run-on paragraph.

**General Findings**:
- minor — the one-time `last-sync.json` migration does not show up in the report.
- minor — `unsupported-recipe` gives no guidance outside the skill and shares exit 4 with conflicts.
- minor — the known reconcile-pull refusal now surfaces as terminal exit 71.

---
*Review generated by /accelerator:review-pr*
