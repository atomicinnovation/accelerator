---
type: "plan-validation"
id: "2026-08-08-0197-accelerator-collaboration-pr-helper-cli-validation"
title: "Validation Report: accelerator-collaboration: PR Helper CLI Implementation Plan"
date: "2026-08-09T21:57:14+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-08-08-0197-accelerator-collaboration-pr-helper-cli"
tags: []
last_updated: "2026-08-09T22:05:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: accelerator-collaboration: PR Helper CLI Implementation Plan

### Implementation Status

✓ Phase 1: Config Catalogue & Personal-File Permission Enforcement -
  Fully implemented
✓ Phase 2: `vcs`/`vcs-adapters` Origin-Remote-URL Parsing - Implemented,
  with a deliberate scope change (see Deviations)
✓ Phase 3: `collaboration` Domain Crate - Implemented, with a deliberate
  scope change (see Deviations)
✓ Phase 4: `collaboration-adapters` - Implemented as a `github` crate
  rather than a generically-named adapters crate (see Deviations)
✓ Phase 5: `github.*` Credential Resolver - Fully implemented, matches
  the plan's own env-first addendum
✓ Phase 6: `collaboration-cli` Sub-binary + Skill Call-site Migration -
  Implemented, with an added `pr` subcommand nesting layer not in the
  plan's code sketch (see Deviations)
✓ Phase 7: Remove the Legacy Bash Scripts and Suites - Fully implemented
✓ Phase 8: User Documentation - Fully implemented

### Automated Verification Results

✓ `mise run cli:check` - passes
✓ `mise run test:unit:cli` - **passes** (1126 tests run: 1126 passed, 0
  skipped). Initially failed on
  `accelerator-corpus::frontmatter_goldens
  this_repositorys_own_corpus_is_clean`: two `EMPTY-PLACEHOLDER`
  violations in
  `meta/reviews/plans/2026-08-08-0197-accelerator-collaboration-pr-helper-cli-review-1.md`
  (`parent: ""` and `relates_to: []` emitted instead of omitted). That
  file was added in the same commit as the plan itself
  (`ae4fc478f715`), predates all eight implementation-phase commits, and
  was unrelated to any of this plan's own code changes — the same class
  of issue was previously hit and fixed on the `0169` branch ("Omit the
  empty relates_to placeholder from the 0169 plan validation
  frontmatter"). Fixed during this validation by dropping the two empty
  keys from that review doc's frontmatter; re-run is green. All 261
  tests specific to this plan's own crates (`collaboration`, `github`,
  `accelerator-collaboration`, `vcs`, `vcs-adapters`, `config`,
  `config-adapters`) also passed cleanly in isolation throughout
  (`cargo nextest run -p collaboration -p accelerator-collaboration -p
  github -p vcs -p vcs-adapters -p config -p config-adapters`: 244
  passed; `-p accelerator-collaboration` alone: 17 passed, including the
  end-to-end mock-server tests).
✓ `mise run build-system:check` - passes
✓ `mise run test:integration:github` - passes (exits 0, discovers 0
  shell suites, floor is 0)
✓ `mise run check` - passes (format + lint + types across all four
  components, including `cargo-pup`'s whole-crate import rule across the
  three new crates, and `lint:dispatch-coherence:check`)

### Code Review Findings

#### Matches Plan:

- Phase 1: `EXTRA_KEYS` carries `github.token`/`github.token_cmd` in
  both the Rust catalogue (`cli/config/src/catalogue.rs:128-129`) and
  its bash mirror (`scripts/config-defaults.sh:215-216`), each with a
  dedicated test. `FileConfigStore`'s personal-file permission/symlink
  enforcement (`cli/config-adapters/src/store.rs:185-193`) is wired into
  both the structured `ReadConfigLevel::read` and the markdown-body
  `ReadContent::config_body` paths, exactly as the plan requires, with
  `Level::Team` reads unaffected and a missing personal file unaffected.
- Phase 5: `cli/collaboration-cli/src/auth.rs:43-91` implements the
  exact env-first precedence (`GH_TOKEN` → `GITHUB_TOKEN` →
  `github.token` config → `github.token_cmd`) with the shared-config
  `token_cmd` ban, matching the plan's post-review Addendum precisely,
  including the doc-comment rationale.
- Phase 6 registration checklist: all of `tasks/shared/paths.py`,
  `tasks/manifest.py`, `.gitignore`, `tasks/build.py`, and
  `tests/integration/tasks/test_github.py` carry the required
  `collaboration` entries.
- Phase 6 skill rewiring: all three of `review-pr/SKILL.md`,
  `respond-to-pr/SKILL.md`, `describe-pr/SKILL.md` call the new binary
  instead of the bash scripts, each invocation is the first line of its
  own fenced block (satisfying the dispatch-coherence guard), and
  `allowed-tools` entries were updated accordingly.
- Phase 7: all five named files plus the `skills/github/scripts/`
  directory itself are gone; `_EXPECTED_GITHUB_SUITES` is `0`; a
  repo-wide grep for the removed scripts' names returns nothing.
- Phase 8: the new `collaboration.md` docs page, sidebar entry, root
  README concept entry, and the `review-a-pr.mdx`/`configuration.md`/
  `configuration-cookbook.md` updates are all present as specified.
- `main.rs`'s `BlockingGitHubClient` shim matches the plan's described
  architecture exactly (one runtime per call, non-nested `block_on`s,
  entering the runtime around `OctocrabClient` construction to satisfy
  `tower::Buffer`'s `tokio::spawn` requirement — a gap the plan itself
  flagged as discovered during implementation).
- `octocrab` is pinned exactly (`=0.54.1`), `deny:check` passes with it
  in the graph, and the hand-rolled mock server records the
  `Authorization` header and proves redirect rejection, matching the
  plan's Phase 4 success criteria.

#### Deviations from Plan:

- **Origin-remote URL parsing moved from `vcs` to `collaboration`, not
  left in `vcs` as Phase 2 specifies.** `cli/vcs/src/origin_remote.rs`
  contains only the `OriginRemote` port (raw remote-URL access); its own
  doc comment states "which hosted forge... and how to parse... are not
  a `vcs` concern." `OwnerRepo`, the `RemoteUrlRecognizer` trait, and
  `resolve_origin_owner_repo` all live in `cli/collaboration/src/lib.rs`
  instead, with `GitHubRemoteUrlRecognizer` (the plan's
  `parse_github_remote_url` equivalent) implemented in the `github`
  crate. This is a coherent architectural improvement (keeps `vcs`
  forge-agnostic) but is a real divergence from Phase 2's and Phase 3's
  literal code sketches, and — unlike Phase 5's precedence reordering —
  is not called out anywhere in the plan text as a reasoned addendum.
- **`collaboration-adapters` does not exist; a `github` crate plays its
  role.** Confirmed via `cli/Cargo.toml`'s workspace members
  (`collaboration`, `github`, `collaboration-cli` — no
  `collaboration-adapters`). The commit message for `8c3ab920443e`
  documents the rationale explicitly (every GitHub-specific concern
  lives in one forge-named crate; a generic adapters crate would be pure
  re-exports). `collaboration`'s own `GitHubApiError` was correspondingly
  renamed to `ForgeApiError` to stay forge-agnostic. The deviation is
  executed coherently — `cli/collaboration/Cargo.toml` depends only on
  `kernel` and `vcs`, no forge dependency leaks into the domain crate —
  but neither the crate rename nor the type rename is reflected
  anywhere in the plan document.
- **The CLI gained an unplanned `pr` subcommand nesting layer.** Phase
  6's code sketch specifies flat `Command::BaseRepo`/`Command::UpdateBody`
  variants (invoked as `accelerator collaboration base-repo`/
  `update-body`). The actual CLI nests these under `Command::Pr { action:
  PrAction }` (`cli/collaboration-cli/src/cli.rs:17-36`), so the real
  invocation is `accelerator collaboration pr base-repo`/`pr
  update-body`. This is applied consistently everywhere it matters
  (`main.rs`, all three rewritten skill call sites), so nothing is
  broken by it, but it makes the plan's own "Desired End State" section
  (lines 168-172, which states the flat command form literally) stale,
  and the change is not documented as a deviation anywhere in the plan.
- Phase 1's `ConfigError` gained no new `InsecurePersonalPermissions`
  variant. Instead, `store::WriteError::InsecurePermissions`
  (`cli/store/src/lib.rs`) is translated by `to_config_error` into the
  existing `ConfigError::Invalid`, which `is_refusal()` already
  classifies correctly. The refusal-semantics intent is fully preserved;
  only the plan's suggested variant name wasn't used.

#### Potential Issues:

- Per-backend URL-form coverage in `vcs-adapters` is thinner than the
  plan's Phase 2 checkbox literally claims: each of `subprocess.rs` and
  `library.rs` has one `a_configured_origin_is_reported` test (a single
  HTTPS URL), not all four forms — a reasonable consequence of moving
  URL parsing out of `vcs` (the four-form coverage now lives in
  `github`'s `remote_url_recognizer.rs` tests, which cover six shapes),
  but the checkbox text as written doesn't literally hold for the `vcs`
  layer alone.
- The plan text itself (Desired End State, Phase 2, Phase 3, Phase 4,
  Phase 6's code sketches) was not updated to reflect the
  `collaboration-adapters`→`github` rename, the `vcs`→`collaboration`
  URL-parsing relocation, or the `pr` subcommand nesting, even though
  Phase 5 demonstrates the plan's own convention for recording this kind
  of reasoned mid-implementation change (its "Addendum" paragraph). A
  reader of this plan after the fact would be misled about the actual
  crate names, type names, and CLI invocation shape without cross-
  checking the code.

### Manual Testing Required:

1. Credential resolution (Phase 5, marked `[ ]` in the plan):
   - [ ] Set `GH_TOKEN`, then `GITHUB_TOKEN`, then `github.token` in
         `config.local.md`, then `github.token_cmd`, individually, and
         confirm each resolves in the stated precedence order.
   - [ ] Configure a `github.token_cmd` in the shared `config.md` and
         confirm the ban error, not silent execution.
2. End-to-end body update (Phase 6, marked `[ ]` in the plan):
   - [ ] `accelerator collaboration pr update-body <pr> --body-file
         <path>` against a real repo updates the PR body (not run in
         the implementation sandbox — no PAT available, and it mutates
         a real PR).
3. Full skill workflows (Phase 6, marked `[ ]` in the plan):
   - [ ] Run `/review-pr`, `/respond-to-pr`, and `/describe-pr`
         end-to-end against a real PR and confirm each completes
         without any fallback to the removed bash scripts.

### Recommendations:

- Update the plan document itself (or add a short closing addendum, in
  the style of Phase 5's) to record the three coherent-but-undocumented
  architectural deviations: `collaboration-adapters` → `github`
  (+`GitHubApiError` → `ForgeApiError`), origin-URL parsing moving from
  `vcs` into `collaboration`, and the `pr` subcommand nesting — so the
  plan stays a trustworthy record of what actually shipped, matching
  the precedent Phase 5 already set for the precedence-order reversal.
- Complete the three remaining manually-deferred checks above when a
  real GitHub PAT and a disposable PR are available.
