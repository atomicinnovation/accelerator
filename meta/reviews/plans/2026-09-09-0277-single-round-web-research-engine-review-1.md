---
type: "plan-review"
id: "2026-09-09-0277-single-round-web-research-engine-review-1"
title: "Plan Review: Single-Round Web Research Engine Implementation Plan"
date: "2026-09-09T23:08:18+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-09-0277-single-round-web-research-engine"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "correctness", "test-coverage", "security", "safety", "compatibility", "standards"]
review_number: 1
review_pass: 2
tags: ["research", "skills", "corpus", "topic-research"]
last_updated: "2026-09-10T00:30:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Single-Round Web Research Engine Implementation Plan

**Verdict:** REVISE

The plan is architecturally disciplined and factually well-anchored: the
`(type, kind)` composite with a `(type, "")` fallback is a proportionate,
backward-compatible, open-closed change; the researcher/reviewer seam is reused
faithfully; and every stated SCHEMA/catalogue/linkage count and line reference
checks out against the tree. It nevertheless needs revision before
implementation — two critical issues (an untrusted-web-content trust boundary
left open in the `researcher` agent, and a `row_for` signature change that
breaks unenumerated callers so Phase 1 will not compile in isolation) plus a
dense cluster of majors concentrated in the `conduct` finding lifecycle
(silent deletion, unreconciled counts, unenforced immutability, non-atomic
set mutation) and the Phase 4 resolver (sub-document resolution, slug recovery
from directory-named sets, an exit-code taxonomy that collides with the
binary's existing convention).

### Cross-Cutting Themes

- **`corpus resolve` exit-code taxonomy collides with the binary's existing
  convention** (flagged by: compatibility, code-quality, architecture,
  correctness) — the new `run_resolve` returns `2 = ambiguous`, but corpus-cli
  already maps every refusal to exit 2 for its four other subcommands, and the
  unregistered-`--type` outcome (the `topic-research`-until-0278 case the skill
  hits on every non-`brief` verb) is assigned no code at all. Four lenses
  independently reached this; it is the single most-reinforced defect.
- **`conduct`'s finding lifecycle desyncs the manifest from disk** (flagged by:
  safety, code-quality, correctness, architecture) — the delete-on-failed-
  validation rule hard-deletes expensive research over a repairable frontmatter
  defect, is not scoped to the just-written file, and never reconciles
  `finding_count`/checkboxes with the findings that actually survived. Because
  0278's indexer keys on this manifest, the divergence surfaces at co-land.
- **The finding document's shape is stated in three unsynchronised places**
  (flagged by: architecture, correctness, standards) — the SCHEMA row, the
  template, and the outputter prose. The actual writer (the no-Bash subagent)
  composes from the prose copy, which no test binds to the schema; the template
  even reproduces `parent: ""`/`relates_to: []` verbatim, the exact shape
  `check_empty_placeholders` rejects.
- **The atomic set-write pattern drops the safeguards that make its precedent
  safe** (flagged by: safety, correctness) — `brief` borrows inventory-design's
  temp-dir-then-rename but onto non-dated `.<slug>.tmp/` and `<slug>/` names,
  without the dated directories or the non-destructive supersede step, so a
  routine re-`brief` collides on both.
- **Load-bearing behaviour rests on manual verification** (flagged by:
  test-coverage, security, safety) — per-kind validation (ADR-0067's whole
  reason to exist), the researcher's no-`Bash` invariant, and the full loop all
  lack an automated regression net.

### Tradeoff Analysis

- **Corpus hygiene vs. preserving expensive research**: `conduct` deletes any
  finding that fails validation "so the set never carries a non-conforming
  document", but each finding is a token-heavy, live-web-dependent artifact.
  Recommendation: quarantine (rename aside with an `.invalid` marker) or
  repair-then-revalidate rather than hard-delete, and treat a wholesale round
  failure as a reported error, not a silent empty set.
- **Iteration ergonomics vs. data-loss safety** on re-`brief`: a bare-slug
  target directory makes re-running natural but risks clobbering a completed
  dossier. Recommendation: refuse-on-existing with a clear message (or adopt
  the precedent's supersede) over silent overwrite; the friction is the safer
  default for a walking skeleton.
- **Simplicity of the researcher-writes-directly model vs. containing an
  untrusted agent**: giving the subagent `Write` keeps `conduct` simple but
  leaves an unmonitored arbitrary-write primitive in an agent consuming
  attacker-controlled input. Recommendation: keep the model but have `conduct`
  snapshot the set and assert the only changed path is the assigned finding.

### Findings

#### Critical

- 🔴 **Security**: Fetched web content is never treated as a trust boundary
  **Location**: Phase 6 (researcher agent, web profile, finding outputter)
  The `researcher` is granted `WebSearch, WebFetch, Write, Read, Grep, Glob, LS`
  and fetches arbitrary attacker-controlled pages, yet nothing instructs it to
  treat that content as data rather than instructions. Injected content can
  redirect it to read local secrets/config and exfiltrate via WebFetch, mistag
  tiers, or write poisoned prose that flows immutably into the finding and then
  into `synthesise`/`synthesis.md`. The corpus already carries the mitigation
  idiom (`skills/vcs/commit/SKILL.md` wraps injected context as untrusted); it
  is not applied here.

- 🔴 **Compatibility**: `row_for` signature change breaks unenumerated callers
  **Location**: Phase 1, sections 1–2 (`SchemaRow` gains `kind` / `validate_file`)
  `row_for` is a pinned public item of the `corpus` crate, and adding a required
  `kind` parameter is backward-incompatible. Phase 1 updates only `validate_file`,
  but `row_for` is also called by `dangling_refs` (`mod.rs:473`) and cross-crate
  by `migrate` (`m0007/backfill.rs:70`, `m0007/rewrite.rs:129,171`). Those
  callers still pass one argument, so `cargo build` fails workspace-wide — Phase 1
  cannot be "green in isolation" as the plan promises.

#### Major

- 🟡 **Security**: The `Write` grant is as unbounded as the rejected `Bash`
  **Location**: Phase 6 (researcher tools grant); Current State ("no safely-scoped shell")
  The plan withholds `Bash` because a subagent grant cannot be scoped, but
  `Write` accepts any absolute path — an injected agent can write `manifest.md`,
  `synthesis.md`, `.accelerator/config.md`, `CLAUDE.md`, or `hooks/`. `conduct`
  re-validates only the expected finding path, detecting neither an out-of-path
  write nor a malicious edit to another set file. The unscoped-grant argument
  that excludes `Bash` applies verbatim to `Write`.

- 🟡 **Compatibility / Code-Quality / Architecture / Correctness**: `corpus resolve` exit-code 2 collides with the binary's refusal=2; unknown-type uncoded
  **Location**: Phase 4, section 3 (CLI registration and exit codes)
  corpus-cli maps every refusal to exit 2 for its four subcommands; `run_resolve`
  returns `2 = ambiguous`, and the unregistered-`--type` outcome is given no code
  in the 0/1/2/3/6 taxonomy. A skill branching on the exit code cannot treat the
  binary uniformly, and unknown-type is indistinguishable from ambiguous — the
  exact path the Phase 7 preamble takes for `topic-research` before 0278.

- 🟡 **Correctness**: Nested-manifest sub-document resolves to its immediate parent, not the set root
  **Location**: Phase 4, section 2 (adapter and containment)
  The rule "the containing directory (the parent of the manifest file)" is wrong
  for sub-documents in subdirectories: design-inventory keeps assets under
  `screenshots/` and topic-research keeps findings under `findings/`. A
  `.../findings/01-x.md` handle returns `findings/`, not the set root — so the
  plan's own manual check ("a sub-document path all resolving to the same set
  root") cannot hold for the only testable nested type.

- 🟡 **Correctness**: `slug::derive` cannot recover a slug from a directory-named set
  **Location**: Phase 4, section 1 (domain algorithm — slug recovery)
  `corpus::slug::derive` starts `filename.strip_suffix(".md")?` and dispatches on
  filename shape; a nested-manifest set root is a directory whose manifest is the
  constant `inventory.md` (derives to `None`). The slug must come from the
  directory name, and design-inventory directories are `YYYY-MM-DD-HHMMSS-{id}` —
  the date-strip leaves `HHMMSS-{id}`, so `--type design-inventory login-page`
  would not match a set whose recovered slug is `143022-login-page`.

- 🟡 **Correctness**: Finding template reproduces the empty-placeholder shape the validator rejects
  **Location**: Phase 5, section 3 (finding template) / Phase 7 conduct validation
  The template literally carries `parent: ""` and `relates_to: []`, but
  `check_empty_placeholders` (`mod.rs:371-383`) rejects any non-`tags` field
  valued `""`/`[]`. If the researcher emits the placeholders verbatim (the most
  literal reading), every finding fails `validate` and is silently deleted —
  yielding a zero-finding set while `conduct` still advances to `researching` and
  `synthesise` has nothing to read.

- 🟡 **Safety / Correctness**: Re-running `brief` on an existing slug collides on the non-dated target
  **Location**: Phase 7, section 2 (brief — atomic set creation)
  `brief` renames `.<slug>.tmp/` onto a bare-slug `meta/research/topics/<slug>/`.
  The cited precedent (inventory-design) uses dated directories plus a
  non-destructive supersede step precisely to avoid this; both are dropped, and
  no behaviour is defined for an existing `<slug>/`. `std::fs::rename` onto a
  non-empty dir fails (platform-dependent) or clobbers — silent loss of a
  completed, expensive-to-reproduce dossier.

- 🟡 **Safety**: A stale `.<slug>.tmp/` from an aborted run is neither detected nor cleaned
  **Location**: Phase 7, section 2 (brief — temp dir handling)
  The temp dir is keyed only on the slug, unlike the precedent's timestamped
  name. If a prior `brief` aborted mid-scoping, a re-run builds *into* the
  leftover dir; `Write` overwrites the files it touches but leaves stale files
  it does not, and the rename-in publishes a contaminated set.

- 🟡 **Safety / Code-Quality / Correctness / Architecture**: `conduct` silently deletes findings and leaves counts unreconciled
  **Location**: Phase 7, section 4 (conduct)
  "A finding that fails validation is deleted" hard-deletes the whole finding
  (body included) over a repairable frontmatter defect, the delete is not scoped
  to the just-written path, and `finding_count`/checkboxes are never reconciled
  with survivors. AC 3 requires `finding_count` to equal findings written and one
  finding per focus area; the silent deletion breaks the invariant and desyncs
  the manifest 0278 indexes.

- 🟡 **Safety**: Finding immutability is asserted but nothing enforces it
  **Location**: Phase 7, section 4 (conduct — output-path allocation)
  Findings are declared immutable "from first write" but the guarantee is
  deferred to 0279 with no mechanism now. A `conduct` re-run after partial
  failure, or an `<nn>` allocation that restarts and collides, lets the
  subagent's `Write` silently overwrite a prior finding — no exists-check, no
  write-once guard.

- 🟡 **Safety / Architecture**: Later verbs mutate the live set non-atomically
  **Location**: Phase 7, sections 3–5 (outline / conduct / synthesise)
  The temp-dir-then-rename protects only `brief`'s initial creation; `outline`,
  `conduct`, and `synthesise` write content and separately edit `manifest.md`
  (`research_status`, the `primary` flip) with no atomicity. A verb failing
  partway leaves the manifest inconsistent with disk (e.g. `primary: synthesis.md`
  pointing at a file that failed to write), observable by a reader or the indexer.

- 🟡 **Architecture**: The finding contract lives in three unsynchronised places
  **Location**: Phase 6 (finding outputter)
  The finding shape is encoded in the SCHEMA row + TSV (Phase 5), the template
  (Phase 5), and the outputter prose (Phase 6); the writer composes from the
  prose while validation enforces the schema, and no test binds the two. The
  outputter also diverges from the review output-format it claims to mirror by
  bundling format + write-sink + contract. Any field change must touch five
  places, only four of them test-checked.

- 🟡 **Code-Quality**: `row_for` collapses unknown-type and unknown-kind into a misleading `InvalidType`
  **Location**: Phase 1, sections 1–2
  A single `None` covers two failures; `validate_file` maps both to
  `InvalidType { found: declared }`. A `topic-research` doc with a typo'd
  `kind: "findng"` reports the valid type as invalid; worse, a future
  `(type, "")`-defaulted multi-kind type would let a typo'd kind fall through and
  validate silently against the wrong required fields and status vocab.

- 🟡 **Test-Coverage**: Per-kind validation enforcement — ADR-0067's core guarantee — is manual-only
  **Location**: Phase 5 Manual Verification vs. Testing Strategy
  The rejections that justify the whole `(type, kind)` change (a `status: draft`
  finding rejected, a `source_profiles`-less brief failing `MISSING-EXTRA`) appear
  only under Manual Verification, though the Testing Strategy claims they are
  automated fixtures. Parity/template-tree tests prove the table is internally
  consistent, not that `validate_file` discriminates on `kind`. Promote to
  committed `validate_file` unit tests and/or `frontmatter_goldens.rs` cases.

- 🟡 **Test-Coverage**: Phase 5 omits the `dump.golden` update its new template keys will regress
  **Location**: Phase 5, section 4 (`TEMPLATE_KEYS` entries)
  `dump.golden` enumerates every template key by name, so five new
  `templates.topic-research-<kind>` entries turn `config_read` red — yet Phase 5's
  Automated Verification lists neither `config_read` nor `the_catalogue_holds`
  (Phase 6 correctly calls out its `agents.researcher` golden row). The phase as
  written cannot pass its own `check`.

- 🟡 **Test-Coverage**: The new resolver exit-code taxonomy is only half covered by goldens
  **Location**: Phase 4, sections 3–4
  The listed goldens exercise resolved (0), ambiguous (2), and outside-root (6);
  invalid→1, not-found→3, and the load-bearing unregistered-type refusal are left
  to manual verification. Half a freshly-built taxonomy has no regression test.

- 🟡 **Test-Coverage**: Skill orchestration and the artifact contract have no automated net independent of live web / 0278
  **Location**: Phase 7 & Testing Strategy (Manual Testing Steps)
  All of `conduct`/`synthesise`'s stateful bookkeeping rests on a manual E2E
  coupled to live WebFetch and the 0278 co-land. The prose cannot be unit-tested,
  but the *artifact contract* can: commit a hand-authored full set
  (manifest/brief/outline/finding/synthesis) as a fixture and assert every
  document validates clean, plus negative cases.

#### Minor

- 🔵 **Security**: The web profile's "arbitrary-URL" family has an unstated SSRF surface
  **Location**: Phase 6, section 2 (web source profile)
  WebFetch can be pointed (including by injected directives) at `file://`,
  `169.254.169.254`, or internal hosts, and the bytes land in a persisted
  finding. State the intended scope (public `http(s)` only) so the boundary is
  explicit; partly bounded by managed-WebFetch guardrails.

- 🔵 **Security**: Reputation tiers are self-assigned from attacker-influenced content
  **Location**: Phase 6, section 2 / Phase 7, section 5
  Tiers are assigned by the agent and carried immutably into `synthesis.md` as
  the corpus's trust signal, with no independent venue check; injected content
  can label a hostile source `tier-1`. Record the source domain alongside each
  tier and derive the tier from venue identity only, so a mistag is auditable.

- 🔵 **Test-Coverage**: No automated guard pins the researcher's tool grant
  **Location**: Phase 6, section 1
  The no-`Bash` invariant rests on prose and a manual check. A future edit adding
  `Bash` to `agents/researcher.md` would pass all automated checks. Add a cheap
  frontmatter assertion pinning the exact `tools:` set.

- 🔵 **Architecture**: Phase-independence claims understate coupling on shared registries
  **Location**: Implementation Approach
  Phases 1/3/5 mutate `SCHEMA`/`templates-schema.tsv`; 2/5/6 the catalogue count
  assertion and its renamed test; 1/3/4/5 the pinned `public-api.txt`; 2/6
  `dump.golden`. "Mutually order-independent" holds only for strictly sequential
  single-branch merges; parallel development conflicts on the same lines. Prefer
  a fixed landing order for these.

- 🔵 **Architecture**: `kind` is semantically overloaded (extra vs. discriminator)
  **Location**: Phase 1
  For work items `kind` is a validated extra with no selection role; for
  topic-research it is the discriminator. The moment a `(work-item, <kind>)` row
  is added (the plan's stated future), every existing `kind: story` silently
  switches from `(work-item, "")` to a kind-specific row — a behaviour change the
  plan does not flag.

- 🔵 **Architecture**: The manifest contract is defined in 0277 but consumed by 0278 with no cross-seam test
  **Location**: Overview / Desired End State
  A mismatch between the manifest shape 0277 writes and the shape 0278's indexer
  expects is latent until both land. Pin the manifest as a shared fixture both
  items reference and enumerate a co-land integration test.

- 🔵 **Code-Quality**: "Mirrors `resolve_bare_number`" risks importing irrelevant complexity
  **Location**: Phase 4, section 1
  `resolve_bare_number` is dense with number-specific concerns (project-code
  prepend, zero-padding, four candidate sources on a `<number>-` prefix) that do
  not apply to a `-<slug>.md` suffix match. Clarify only the *shape* is mirrored
  and specify the slug-suffix rule directly.

- 🔵 **Code-Quality**: `SubPath` as a domain `InputClass` blurs the domain/adapter boundary
  **Location**: Phase 4, sections 1–2
  Whether an input is a sub-document, a set directory, or an arbitrary path is a
  filesystem fact; the mirrored `work::resolve` domain is filesystem-free. Keep
  the classifier syntactic (Path/Slug/Invalid) and let the adapter own the
  file/dir/sub-path branching.

- 🔵 **Code-Quality / Compatibility**: The "13 call sites" for the template resolver is inaccurate
  **Location**: Phase 1, section 5
  `template::resolve`'s callers are launcher-internal (~5:
  `inbound/cli.rs:242,253,582,606` plus the internal `core/template.rs:44`); 13
  is the SCHEMA row count, conflated. Re-enumerate so no caller is missed when
  threading `None`.

- 🔵 **Code-Quality**: Inserting `kind` mid-row shifts positional indices across two divergent TSV parsers
  **Location**: Phase 1, sections 3–4
  `template_shape.rs::parse_schema_tsv` indexes positionally while
  `schema.rs` destructures; a mid-row insertion shifts five subsequent columns in
  the indexed parser. Either append `kind` as the trailing column or spell out
  each shifted `fields[N]`.

- 🔵 **Compatibility**: The pinned-surface delta is understated
  **Location**: Phase 1, section 6 / Phase 3, section 4
  Phase 3 also moves `TYPE_PAIRS` (a pinned `[…; 16]`); Phase 1 adds
  `TemplateRow::kind` and shifts `SCHEMA_TAB_FIELDS`, none listed. `update`
  regenerates the whole snapshot so `check` still passes, but a reviewer sees
  more churn than described. List every pinned item each phase moves.

- 🔵 **Correctness**: The containment check must canonicalise the root too
  **Location**: Phase 4, section 2
  work-cli canonicalises the root (`canonical_work_dir`) precisely because a
  canonicalised candidate carries the macOS `/private` prefix a raw root lacks.
  If the root is left as `project_root.join(dir)`, `starts_with` fails for
  in-root paths → false outside-root (exit 6). Canonicalise the root once and add
  a symlinked-temp-dir golden.

- 🔵 **Correctness**: The outline rubric's "10+" band contradicts the `breadth: 8` ceiling
  **Location**: Phase 7, section 3 (outline)
  Handing the model a rubric that names a count above the hard ceiling is
  internally contradictory; a model following the "10+" band violates AC 2/AC 8.
  Cap the rubric at 8 explicitly or drop the unreachable band this slice.

- 🔵 **Standards**: `allowed-tools` deviates from the block-list, narrow-scope convention
  **Location**: Phase 7, section 1
  Every existing skill writes `allowed-tools` as a YAML block sequence and scopes
  `Bash` narrowly; the plan uses an inline comma-separated scalar (parses as one
  string) with a broad `Bash(accelerator corpus *)` wildcard. Render as a block
  list enumerating the specific subcommands used.

- 🔵 **Standards**: The example finding template omits the per-field inline comments every shipped template carries
  **Location**: Phase 5, section 3
  The shown template drops the type-discriminator, id-derivation, status-vocab,
  and typed-linkage-slot comments the other templates carry — contradicting the
  plan's own prose ("status vocab as an inline comment"). Carry the established
  annotations into all five.

- 🔵 **Standards**: Leaf-naming asymmetry between `profiles/web` and `outputters/finding-outputter`
  **Location**: Phase 6, sections 2–3
  The review-side precedent suffixes the category into both leaf names
  (`<x>-lens`, `<x>-output-format`); one of the two new categories breaks it, and
  a global `name: web` is collision-prone. Align the two.

#### Suggestions

- 🔵 **Architecture**: The duplicated resolver algorithm is an accepted ADR-0068 tradeoff
  **Location**: Phase 4, section 1
  Mirroring over sharing is deliberate and the domains genuinely differ; if a
  third resolver appears, factor the doc-type-agnostic pieces (containment,
  candidate tagging) into a small shared helper to bound drift.

- 🔵 **Correctness**: Enumerate the `TYPE_PAIRS` triples for topic-research
  **Location**: Phase 3, section 3
  Phase 3 says "add the topic-research entries" without naming the triples;
  undefined triples risk classifying valid references as `Ambiguous`. Enumerate
  them and note `TYPE_PAIRS` contributes to the snapshot delta.

- 🔵 **Standards**: Note the Rust snippets are illustrative and rustfmt-wrapped to 80 columns
  **Location**: Phase 1
  A couple of illustrative lines exceed the enforced `max_width = 80`; harmless
  (rustfmt reflows on implementation) but worth a note so the illustration
  matches the shipped shape.

### Strengths

- ✅ The `(type, kind)` composite with a `(type, "")` fallback is backward-
  compatible and open-closed: all 13 existing types — and work items that
  already carry a non-empty `kind` — resolve unchanged, and new kinds are data
  rows, not code branches.
- ✅ Reuses the ADR-0005 path-passing generic-agent seam (reviewer → researcher),
  and the no-`Bash` researcher with `conduct`-owned validation places the CLI
  trust boundary sensibly, mirroring the review-plan/reviewer split.
- ✅ The `corpus resolve` containment check faithfully mirrors work-cli
  (canonicalise + `starts_with`), and the base exit-code taxonomy matches
  `work resolve` exactly (0/1/2/3/6).
- ✅ Strong scoping discipline: an explicit phase→acceptance-criterion map, a
  detailed "What We're NOT Doing" against 0278–0284, and awareness of the
  cross-cutting invariants (public-api snapshot, catalogue count, TSV/SCHEMA
  parity) most plans miss.
- ✅ Factual anchors verified accurate against the tree: SCHEMA length 13,
  catalogue 55, `LINKAGE_SOURCE_TYPES` 14, `TYPE_PAIRS` 16, the cited line
  references, and `from_linkage_type_name("topic-research") → None` until 0278.
- ✅ The Rust phases carry a well-proportioned, existing-pattern test strategy
  (unit + black-box goldens + parity + per-phase public-api snapshot).
- ✅ Deriving finding metadata once in `conduct` and pre-allocating distinct
  output paths avoids per-agent time skew and concurrent write collisions.
- ✅ Fan-out is bounded (`breadth: 8`, inert `depth: 1`) and the researcher
  returns summaries not bodies, keeping `conduct`'s context bounded.
- ✅ Modelling the 0277↔0278 simultaneity as `relates_to` rather than a
  reciprocal `blocked_by` correctly avoids a `blocks`/`blocked_by` cycle.
- ✅ File placement and naming follow established directory conventions, and
  Phase 4 correctly scopes the resolver as a subcommand on the existing `corpus`
  binary, sidestepping the thirteen-point sub-binary registration checklist.

### Recommended Changes

1. **Update every `row_for` caller in Phase 1, or keep the signature additively
   compatible** (addresses: `row_for` signature change breaks unenumerated
   callers). Either thread `kind` through `dangling_refs` and the three `migrate`
   m0007 sites (plus tests) in Phase 1, or keep `row_for(type)` delegating to a
   new `row_for_kind(type, kind)` so the pinned signature and the `migrate`
   consumer are untouched.

2. **Add an untrusted-content contract to the web profile and finding outputter,
   and have `conduct` contain the subagent's writes** (addresses: web-content
   trust boundary; unbounded `Write` grant; SSRF surface). State that fetched
   content is data never instructions; forbid reading/transmitting repository
   files outside the injected task; scope the URL family to public `http(s)`;
   and have `conduct` snapshot the set before spawning and reject a round if any
   path other than the assigned finding changed.

3. **Define the full `corpus resolve` exit-code contract, including a distinct
   unknown-type code** (addresses: exit-code collision). Reserve a code outside
   {0,1,2,3,6} for unregistered `--type` (or document reusing 1 and why), state
   that resolve deliberately diverges from the binary's refusal=2, and add
   goldens for invalid→1, not-found→3, and unknown-type.

4. **Specify `conduct`'s finding lifecycle precisely** (addresses: silent
   deletion; unreconciled counts; unenforced immutability; non-atomic mutation).
   Quarantine or report a failed finding rather than hard-deleting; scope any
   deletion to the just-written path; derive `finding_count`/checkboxes from
   retained findings; refuse to overwrite an existing finding path; and flip the
   manifest last so a partial failure leaves the prior consistent state.

5. **Fix the Phase 4 resolver's set-root recovery** (addresses: sub-document
   resolution; slug recovery from directory sets; root canonicalisation). Walk up
   to the first segment under the type directory (not the immediate parent);
   define how a slug is recovered from a set *directory* name and reconcile the
   design-inventory `HHMMSS` segment; and canonicalise the configured root before
   `starts_with`. Add goldens for a deeper sub-document and an in-root symlinked
   path.

6. **Make per-kind validation and the artifact contract automated** (addresses:
   per-kind validation manual-only; no artifact-contract net; Phase 1/5 seam).
   Commit `validate_file` unit tests and/or `frontmatter_goldens.rs` cases for a
   rejected `draft` finding, a passing `complete` finding, and a
   `source_profiles`-less brief; commit a hand-authored full set as a fixture
   asserting every document validates clean.

7. **Fix the empty-placeholder trap and the `brief` re-run collision**
   (addresses: template reproduces rejected shape; re-`brief` clobbers; stale
   temp dir). Show the linkage keys as omitted/commented rather than
   `""`/`[]`; refuse (or supersede) an existing `<slug>/` and clean the temp dir
   on a failed rename; and either uniquely name or fail-fast on a stale
   `.<slug>.tmp/`.

8. **Add the omitted Phase 5 `dump.golden`/catalogue steps and verification**
   (addresses: Phase 5 golden regression). Insert the five
   `templates.topic-research-<kind>` rows into `dump.golden` and list
   `config_read` + `the_catalogue_holds` in Phase 5's Automated Verification.

9. **Correct the plan's smaller accuracy and convention gaps** (addresses:
   misleading `InvalidType`; "13 call sites"; understated snapshot delta;
   overloaded `kind`; phase-independence coupling; `allowed-tools` shape; template
   annotations; leaf naming; the "10+"/8 rubric contradiction). These are
   individually minor but cheap to fix and several affect whether a phase
   compiles or lints clean.

## Per-Lens Results

### Architecture

**Summary**: Architecturally disciplined — a proportionate, backward-compatible
`(type, kind)` composite, faithful reuse of the ADR-0005 generic-agent seam, and
a sensibly-placed trust boundary. Two structural weaknesses stand out: the
finding contract is represented in three unsynchronised places with the writer
using the untested prose copy, and `conduct` writes findings into the live set
with a `finding_count` that can diverge from disk.

**Strengths**:
- The `(type, kind)` composite with `(type, "")` fallback is a clean,
  backward-compatible, open-closed extension whose cost matches the concrete
  five-shape problem.
- Reusing the path-passing generic-agent pattern (reviewer → researcher) with a
  no-`Bash` researcher and `conduct`-owned validation places the trust boundary
  sensibly.
- Strong scoping: an explicit phase→AC map, a detailed non-goals boundary, and
  awareness of cross-cutting invariants most plans miss.
- Modelling 0277↔0278 as `relates_to` avoids a `blocks`/`blocked_by` cycle.

**Findings**:
- 🟡 (major, medium) Phase 6 — the finding shape lives in the SCHEMA row, the
  template, and the outputter prose; the writer composes from prose while
  validation enforces the schema, and no test binds them. The outputter also
  diverges from the sink-free review output-format. A field change touches five
  places, four test-checked. Suggest a single contract source the outputter
  references.
- 🟡 (major, medium) Phase 7 conduct — `finding_count: <n>` (focus-area count)
  is never reconciled with delete-on-validation, and web-fetch failure has no
  retry/partial-round accounting; unlike `brief`, `conduct` writes into the live
  set. The manifest 0278 indexes can claim more findings than exist. Derive the
  count from validated findings and stage writes before the manifest update.
- 🔵 (minor, high) Implementation Approach — "mutually order-independent" phases
  share global registries (catalogue count, SCHEMA/TSV, public-api, dump.golden);
  independent only for sequential single-branch merges. Prefer a fixed order.
- 🔵 (minor, medium) Overview — the manifest contract is defined/tested in 0277
  but consumed by 0278 with no test spanning the seam until co-land. Pin a shared
  manifest fixture and a co-land integration test.
- 🔵 (minor, medium) Phase 4 — `run_resolve` bypasses the shared `Outcome→report`
  path; exit 2 means refusal elsewhere but ambiguous here, and unregistered-type
  refuses into 2. Route through a shared taxonomy or document the divergence.
- 🔵 (minor, medium) Phase 1 — `kind` is overloaded (work-item extra vs.
  topic-research discriminator); a future `(work-item, <kind>)` row silently
  re-discriminates every existing work item. Flag as a behaviour change needing
  its own migration analysis.
- 🔵 (suggestion, low) Phase 4 — mirroring `work::resolve` duplicates a subtle
  algorithm; an accepted ADR-0068 tradeoff. Factor shared helpers if a third
  resolver appears.

### Code-Quality

**Summary**: Unusually disciplined — mirrors established patterns, keeps the
`(type, kind)` change behaviour-preserving, extracts a clean `resolve_one` seam,
injects a `DirectoryLister` port, and the Rust snippets are comment-free and
TDD-framed. Main risks: error categorisation (`row_for` collapses unknown
type/kind; `conduct` silently deletes findings) and a second, divergent
exit-code convention in one binary.

**Strengths**:
- Every phase is framed red-green-refactor with the failing test named first,
  and the `(type, kind)` change is explicitly behaviour-preserving.
- The template resolver change is a clean extract-and-orchestrate (`resolve_one`
  keeps the untouched tier-walk).
- Resolution reuses an injected `DirectoryLister` port and a pure-domain
  classifier, preserving testability.
- Snippets honour repo conventions — no comments, rich domain naming, parity
  self-checks carried forward.

**Findings**:
- 🟡 (major, medium) Phase 1 — `row_for` returns one `None` for unknown-type and
  unknown-kind; `validate_file` maps both to `InvalidType`. A typo'd `kind`
  reports a valid type as invalid, and a future `(type,"")`-defaulted type would
  silently mis-validate. Emit a dedicated unknown-kind violation.
- 🟡 (major, medium) Phase 7 conduct — silently deletes a finding (a ~15× chat
  artifact) on validation failure, with no stated checkbox/`finding_count`
  reconciliation and no diagnostic. Specify the reconciliation and report rather
  than delete.
- 🟡 (major, medium) Phase 4 — a second exit-code convention (`ExitCode`
  directly) beside the shared `Outcome→report`; ambiguous=2 collides with the
  existing refusal=2 and the unregistered-type code is undefined. Unify or make
  the two conventions explicit and non-overlapping.
- 🔵 (minor, medium) Phase 4 — "mirrors `resolve_bare_number`" risks importing
  number-specific candidate machinery irrelevant to slug-suffix matching.
- 🔵 (minor, medium) Phase 4 — `SubPath` as a domain `InputClass` smuggles
  filesystem knowledge into the filesystem-free domain; keep the classifier
  syntactic and branch in the adapter.
- 🔵 (minor, high) Phase 1 — "13 call sites" for `template::resolve` is wrong
  (~5, launcher-internal); 13 is the SCHEMA row count.
- 🔵 (minor, medium) Phase 1 — inserting `kind` mid-row shifts positional indices
  in `parse_schema_tsv`; append it as the trailing column or spell out each index.

### Correctness

**Summary**: The plan's factual anchors check out (SCHEMA 13, catalogue 55,
`LINKAGE_SOURCE_TYPES` 14, `TYPE_PAIRS` 16, the line references), and the
`(type, kind)` fallback and exit-code base are sound and backward-compatible.
The most significant risks cluster in Phase 4's resolver (sub-document set-root
recovery; `slug::derive` against directory-named sets) plus `conduct`'s
count/checkbox invariant and an empty-placeholder validation trap the templates
reproduce.

**Strengths**:
- The exit-code taxonomy faithfully mirrors work-cli's verified mapping
  (FAILURE=1, USAGE=2, NOT_FOUND=3, OUTSIDE=6).
- The `(type, kind)` fallback is correctly backward-compatible, including
  work items that carry a non-empty `kind`.
- Stated line numbers and counts are accurate, and
  `from_linkage_type_name("topic-research")` correctly returns `None` until 0278.
- Deriving finding metadata once and pre-allocating output paths avoids time
  skew and write collisions.
- Every pinned-surface change calls out regenerating the public-api snapshot.

**Findings**:
- 🟡 (major, high) Phase 4 — a nested-manifest sub-document resolves to its
  immediate parent (`findings/`, `screenshots/`), not the set root; the plan's
  own sub-document manual check cannot hold. Walk up to the first segment under
  the type directory and add a deeper-nested golden.
- 🟡 (major, medium) Phase 4 — `slug::derive` requires a `.md` filename and
  returns `None` for `inventory.md`; a directory-named set has no `.md`, and
  design-inventory's `HHMMSS` segment survives the date-strip so the recovered
  slug won't match a user's descriptive slug. Define directory-name slug recovery.
- 🟡 (major, medium) Phase 7 conduct — `finding_count`/checkboxes are never
  reconciled with delete-on-validation, breaking AC 3 and desyncing the manifest
  0278 indexes. State the invariant precisely and decide retry vs. reported
  shortfall.
- 🟡 (major, medium) Phase 5/7 — the finding template carries `parent: ""`/
  `relates_to: []`, which `check_empty_placeholders` rejects; verbatim emission
  fails every finding, which are then silently deleted into an empty set. Show
  the keys omitted, and error on wholesale failure.
- 🔵 (minor, medium) Phase 4 — the unregistered-type outcome has no code in
  0/1/2/3/6 and collides with refusal=2/invalid=1. Reserve a distinct code.
- 🔵 (minor, medium) Phase 4 — the containment check must canonicalise the root
  too (macOS `/private`), not just the candidate, or in-root paths mis-classify
  as outside-root.
- 🔵 (minor, medium) Phase 7 brief — `fs::rename` onto an existing bare-slug dir
  fails or clobbers; specify behaviour and clean the temp dir on failure.
- 🔵 (minor, medium) Phase 7 outline — the rubric's "10+" band contradicts the
  `breadth: 8` hard ceiling; a model following it violates AC 2/AC 8.
- 🔵 (suggestion, low) Phase 3 — `TYPE_PAIRS` triples for topic-research are
  unenumerated (undefined triples risk `Ambiguous`), and `TYPE_PAIRS` itself
  moves the snapshot.

### Test-Coverage

**Summary**: The Rust phases carry a genuine, well-proportioned strategy
mirroring repo patterns. The central weakness is that ADR-0067's core guarantee
— that `kind` selects a distinct row and enforces per-kind status/fields — is
verified only manually, contradicting the Testing Strategy's own claim. Combined
with an incomplete exit-code taxonomy, a missing `dump.golden` update, and the
whole loop resting on a deferred live-web manual E2E, several load-bearing
behaviours lack a regression net.

**Strengths**:
- Phases 1–5 have a sound unit-plus-integration pyramid (row_for tests, TSV/SCHEMA
  parity, template_shape_tree, config-read goldens, resolver domain + goldens,
  per-phase public-api snapshot).
- The resolver is tested against real proxy types (design-inventory, codebase-
  research) plus an outside-root refusal — a sensible substitution.
- The five templates gain per-kind shape coverage via `template_shape_tree`.
- Manual steps are concrete (exact status transitions, the primary flip, counts).
- The live-web/0278 manual-only risk is explicitly acknowledged.

**Findings**:
- 🟡 (major, high) Phase 5 — per-kind rejection (draft finding, source_profiles-
  less brief) is manual-only though claimed automated; parity tests don't prove
  `validate_file` discriminates on `kind`. Promote to committed tests.
- 🟡 (major, high) Phase 5 — the five new template keys regress `dump.golden`,
  but the phase lists neither `config_read` nor `the_catalogue_holds`; it can't
  pass its own `check`.
- 🟡 (major, medium) Phase 4 — only 3 of the exit-code outcomes have goldens;
  invalid→1, not-found→3, and unregistered-type are untested.
- 🟡 (major, medium) Phase 7 — the artifact contract has no automated net
  independent of live web/0278; commit a hand-authored full set fixture.
- 🔵 (minor, medium) Phase 1/5 — the discriminating path ships in Phase 1 with
  tests that never discriminate (SCHEMA stays 13); close the seam in Phase 5.
- 🔵 (minor, medium) Phase 6 — no automated guard pins the researcher's `tools:`
  (no `Bash`); add a frontmatter assertion.

### Security

**Summary**: The central surface is the Phase 6 `researcher` subagent — arbitrary
WebFetch plus `Write` and whole-tree read. The plan reasons carefully about
excluding `Bash` but never treats fetched web content as a trust boundary: no
injection handling, a `Write` grant as unbounded as the rejected `Bash`, and a
post-write check that inspects only the expected path. The containment check and
the `Bash`-exclusion reasoning are sound; the injection/exfiltration/unbounded-
write gaps are not.

**Strengths**:
- The `corpus resolve` containment check faithfully mirrors work-cli
  (canonicalise both sides + `starts_with`), closing path-traversal and symlink
  escape, with explicit macOS `/private` handling in the goldens.
- The `Bash`-exclusion reasoning is correct (a skill's `allowed-tools`
  auto-approves; the agent ceiling admits only bare `Bash`), keeping CLI
  ownership in `conduct`.
- The resolve outcome is root-confined with an explicit outside-root code.

**Findings**:
- 🔴 (critical, high) Phase 6 — fetched content is never marked untrusted;
  injection can drive local-secret reads + WebFetch exfiltration, tier mistagging,
  or poisoned prose flowing into `synthesise`. Add an untrusted-content contract.
- 🟡 (major, high) Phase 6 — `Write` accepts any path; the agent can write
  `manifest.md`, config, `CLAUDE.md`, or `hooks/`, and `conduct` re-validates
  only the finding path. Snapshot the set and assert only the assigned path
  changed.
- 🔵 (minor, medium) Phase 6 — "arbitrary-URL" WebFetch has an unstated SSRF
  surface (`file://`, `169.254.169.254`, internal hosts). State public-`http(s)`
  scope.
- 🔵 (minor, medium) Phase 6/7 — self-assigned tiers from attacker content are
  carried immutably into `synthesis.md`; record the source domain and derive the
  tier from venue identity only.

### Safety

**Summary**: The initial set creation borrows a sound temp-dir-then-rename
publish but strips the two safeguards that make its precedent safe (dated
directory names and a non-destructive supersede step). Re-running `brief`
collides on both non-dated names; later verbs mutate the published set with no
atomicity; findings are declared immutable but nothing enforces it; and
delete-on-failed-validation hard-deletes repairable research with no recovery.

**Strengths**:
- Initial creation uses an atomic directory rename aligned with the indexer's
  dot-skipping lister, closing the half-written-set window for `brief`.
- Fan-out is bounded (`breadth: 8`, inert `depth: 1`).
- Every written document is validated after writing.
- The researcher returns summaries, keeping `conduct` bounded.
- The `(type, kind)` change is backward-compatible with no data migration.

**Findings**:
- 🟡 (major, high) Phase 7 brief — re-running on the same subject collides on the
  non-dated `<slug>/`; rename fails or clobbers a completed dossier. Refuse or
  supersede.
- 🟡 (major, medium) Phase 7 brief — a stale `.<slug>.tmp/` is neither detected
  nor cleaned; a re-run builds into it and publishes contamination. Fail-fast or
  use a per-run unique temp name.
- 🟡 (major, medium) Phase 7 conduct — delete-on-failed-validation hard-deletes
  the finding body over a repairable defect, unscoped. Quarantine/repair;
  constrain deletion to the just-written path.
- 🟡 (major, medium) Phase 7 conduct — nothing enforces finding immutability; a
  re-run or `<nn>` collision overwrites an existing finding. Refuse-on-existing;
  derive `<nn>` from existing findings.
- 🟡 (major, medium) Phase 7 outline/conduct/synthesise — later verbs mutate the
  published set non-atomically; a partial failure leaves the manifest
  inconsistent with disk. Flip the manifest last; reconcile counts after
  deletions.

### Compatibility

**Summary**: The central contract change — reshaping the pinned `row_for` from
`(type)` to `(type, kind)` — is backward-incompatible on a crate whose surface
is pinned and consumed cross-crate, yet Phase 1 enumerates only one caller, so
the phase would not compile. The `(type, "")` fallback is otherwise genuinely
backward-compatible, the linkage additions are correctly decoupled from
`DocTypeKey`, and the resolve taxonomy mirrors work resolve but collides with
the binary's refusal=2.

**Strengths**:
- The `(type, "")` fallback is verifiably backward-compatible, including
  kind-carrying work items.
- `topic-research` is correctly kept out of `DOC_TYPES`; the membership-only
  linkage arrays land green in isolation.
- The resolve exit-code base mirrors work resolve exactly.
- No new Claude Code version floor (the same subagent/skill-preload mechanism).
- Every pinned-surface change schedules a snapshot regen.

**Findings**:
- 🔴 (critical, high) Phase 1 — `row_for` is pinned and also called by
  `dangling_refs` and cross-crate by `migrate` m0007 (`backfill.rs:70`,
  `rewrite.rs:129,171`); updating only `validate_file` fails the build
  workspace-wide. Update all callers or keep the signature additively compatible.
- 🟡 (major, high) Phase 4 — resolve's exit 2 (ambiguous) collides with the
  binary's refusal=2, and the unregistered-type outcome has no code. Define the
  full taxonomy including a distinct unknown-type code.
- 🔵 (minor, medium) Phase 3/1 — the pinned-surface delta is understated
  (`TYPE_PAIRS` length, `TemplateRow::kind`, `SCHEMA_TAB_FIELDS`). List them.
- 🔵 (minor, low) Phase 1 — "13 call sites" for `template::resolve` looks
  conflated with the SCHEMA count (~5 launcher-internal callers).

### Standards

**Summary**: Strongly aligned with repo conventions: file placement mirrors
existing directories, `(type, kind)` naming follows ADR-0067, Phase 4 is
correctly scoped to sidestep the sub-binary checklist, and every pinned-crate
phase regenerates the snapshot. The Rust snippets honour the low-comment
tolerance, and the template inline comments are the established convention, not a
violation. A few consistency gaps remain around the skill's `allowed-tools`, the
under-annotated example template, and profile/outputter leaf naming.

**Strengths**:
- Phase 4 is correctly scoped as a subcommand on existing crates, so neither
  registration checklist applies; corpus's snapshot vs. corpus-cli's exemption
  are identified correctly.
- File placement follows directory conventions exactly.
- Template and config-key naming follow ADR-0067 consistently.
- The low-comment tolerance is honoured; the template frontmatter comments are
  the established authoring convention.
- Phase independence and TDD-per-phase are stated and mapped to ACs.

**Findings**:
- 🔵 (minor, medium) Phase 7 — `allowed-tools` is an inline scalar with a broad
  `Bash(accelerator corpus *)` wildcard; every existing skill uses a block list
  with narrow scoping. Render as a block list enumerating the subcommands used.
- 🔵 (minor, medium) Phase 5 — the example finding template omits the per-field
  inline comments every shipped template carries (and the plan's own prose calls
  for). Carry them into all five.
- 🔵 (minor, medium) Phase 6 — leaf-naming asymmetry (`profiles/web` vs.
  `outputters/finding-outputter`); `name: web` is collision-prone. Align them.
- 🔵 (suggestion, medium) Phase 1 — a couple of illustrative Rust lines exceed
  the enforced `max_width = 80`; note they are illustrative / rustfmt-wrapped.

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** APPROVE

The pass-2 assessment below concluded COMMENT (sound, with accepted deferrals);
the reviewer approved the plan for implementation on that basis, so the recorded
verdict is APPROVE and the plan is marked `ready`.

All 8 lenses re-ran against the revised plan. Both prior 🔴 criticals and every
prior 🟡 major are resolved — the untrusted-content contract, the enumerated
`row_for` callers, the walk-up set-root resolver, the quarantine-not-delete
lifecycle, the write-then-flip-manifest ordering, the promoted per-kind tests, and
the exit-code-4 taxonomy all landed. The pass surfaced a fresh set of
mostly-medium-confidence issues, several a direct consequence of the round-1 edits
(quarantine marker collision, `dangling_refs kind=""`, the caller undercount) and
several convergent across 3–4 lenses. These were addressed in a round-2 edit; the
residual is accepted deferrals plus minor/suggestion polish, hence COMMENT rather
than a further REVISE.

### Previously Identified Issues

- 🔴 **Security** — fetched web content not a trust boundary — **Resolved** (contract
  on researcher/web-profile/outputter; SSRF scope; venue-identity tiers). Security
  now lists these as strengths.
- 🔴 **Compatibility** — `row_for` signature breaks unenumerated callers — **Resolved**
  at the design level (callers enumerated); the round-1 wording undercounted them and
  introduced a `kind=""` regression, both corrected in round 2 (see below).
- 🟡 **Exit-code taxonomy collision** (compat/code-quality/arch/correct) — **Resolved**
  (distinct code 4; round-2 edit routes the whole binary through one `exit_codes`
  module).
- 🟡 **`conduct` silent delete + count desync** (safety/code-quality/correct/arch) —
  **Resolved** (quarantine + reconcile-to-retained).
- 🟡 **Sub-document set-root resolution** (correctness) — **Resolved** (walk up to the
  first segment under the type dir).
- 🟡 **`slug::derive` from directory sets** (correctness/arch) — **Partially → Resolved**
  in round 2 (dedicated directory-name rule defined against both naming conventions,
  HHMMSS pinned).
- 🟡 **Empty-placeholder template trap** (correctness) — **Resolved** (write-time omit
  rule in the outputter; template restored to shipped `parent: ""`/`relates_to: []`
  form in round 2 per the standards correction).
- 🟡 **Re-`brief` overwrite / stale `.tmp`** (safety/correct) — **Resolved**
  (refuse-on-existing; stale-tmp cleanup).
- 🟡 **Finding immutability unenforced** (safety) — **Resolved** (refuse-on-existing
  finding path).
- 🟡 **Non-atomic later verbs** (safety/arch) — **Resolved** (write-then-flip;
  round-2 adds manifest re-validation after each edit).
- 🟡 **Finding contract in three places** (arch) — **Resolved** (outputter carries one
  example block bound to the schema row by a parse-and-compare test).
- 🟡 **Misleading `InvalidType`** (code-quality) — **Resolved** (`UnknownKind` split;
  round-2 adds it to `structural_gate_failed` and a committed test).
- 🟡 **Per-kind validation manual-only / missing `dump.golden` / partial resolver
  goldens / no artifact-contract net** (test-coverage) — **Resolved** (committed tests;
  `dump.golden` step; full 0/1/2/3/4/6 goldens; round-2 wires the full-set fixture into
  Phase 5 as an explicit `--file` test).
- Prior minors (SSRF, tier integrity, no-`Bash` guard, phase-coupling, `kind`-overload,
  snapshot delta, root canonicalisation, rubric 10+/8, `allowed-tools`, template
  annotations, leaf naming, `resolve_bare_number`/`SubPath`) — **Resolved**.

### New Issues Introduced (surfaced pass 2; disposition in round-2 edit)

- 🟡 **Security** — `dangling_refs kind=""` silently skips topic-research integrity
  (also compat) — **Fixed**: reads the document's real `kind`.
- 🟡 **Compatibility** — `row_for` caller enumeration undercounted — **Fixed**:
  generalised to "the compiler enumerates the full set" + `structural_gate_failed`.
- 🟡 **Safety/arch/code-quality** — quarantine `.invalid` marker (collision, not
  dot-prefixed for the indexer, index-allocation interaction) — **Fixed**:
  dot-prefixed, uniquely-suffixed, overwrite-guarded; index scans markers; fixture
  carries a quarantined case.
- 🟡 **Safety/security/correct/test-cov** — missing-finding-write case undefined —
  **Fixed**: distinct outcome (report, checkbox unflipped, excluded from count).
- 🟡 **Safety** — partial-round crash duplicates/under-claims — **Fixed**:
  reconcile-against-disk at run start; identity-keyed allocation.
- 🟡 **Security** — second-order injection via `synthesise` re-ingest — **Fixed**:
  untrusted-content framing extended to `synthesise`/`conduct` finding reads.
- 🟡 **Security** — `WebFetch` + repo-wide read exfiltration channel — **Fixed**:
  researcher `tools:` trimmed to `WebSearch, WebFetch, Write, Read`.
- 🟡 **Test-coverage** — `UnknownKind` untested; full-set fixture unwired — **Fixed**:
  committed cases added to Phase 5.
- 🔵 Minors (`TemplateRow.kind` parity, `agents.golden`, manifest re-validation,
  precondition guards, outputter drift-test spec, lookalike domains, `kind`-overload
  guard test) — **Fixed** in round 2.
- ⚪ **Deferred (accepted)**: the `conduct` write-scope assertion and cross-verb
  transactional atomicity, with human commit review of the git-tracked tree recorded
  as the compensating control and live-web-in-unattended-context gated on the assertion
  landing.

### Assessment

The plan is in good shape to implement. The substantive design is sound and the
round-1 review's blockers are closed; the pass-2 issues were second-order and are
addressed, with the one remaining risk (the researcher's unbounded `Write`) an
explicitly-accepted, compensated deferral. No further review pass is required before
implementation.

---
*Review generated by /accelerator:review-plan*
