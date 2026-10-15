---
type: "codebase-research"
id: "2026-10-13-0332-push-pr-descriptions-to-github-unwrapped"
title: "Push PR Descriptions to GitHub Unwrapped"
date: "2026-10-13T19:20:34+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0332"
parent: "work-item:0332"
topic: "Push PR Descriptions to GitHub Unwrapped"
tags: ["research", "codebase", "collaboration", "collaboration-cli", "github", "describe-pr", "markdown", "pulldown-cmark"]
revision: "d200d2ddea8da22508fb055f10190ded038c0c09"
repository: "accelerator"
last_updated: "2026-10-13T19:20:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Push PR Descriptions to GitHub Unwrapped

**Date**: 2026-10-13 19:20 UTC
**Author**: Toby Clemson
**Git Commit**: d200d2ddea8da22508fb055f10190ded038c0c09
**Branch**: detached working copy (not on `main`, not pushed)
**Repository**: accelerator

## Research Question

What does the codebase look like for work item 0332? The item unwraps GFM
soft line breaks in the body that `accelerator collaboration pr update-body`
PATCHes to GitHub, and leaves the `meta/prs/` artefact wrapped. The research
covers:

- where the change lands
- what the dependency and architecture gates demand
- how `pulldown-cmark` behaves on the story's fixtures
- what else must change

## Summary

- **Seam.** The body travels as a plain `&str`:
  - `run_update_body` reads it with `read_to_string`.
  - It passes the string to the domain function `update_pull_request_body`.
  - The domain function hands it to the `PullRequestBodyUpdate` port.
  - `OctocrabClient::update_body` sends it, and octocrab serialises
    `{"body": …}`.

  The repository builds no JSON itself, and the body file is never written
  back. That makes the artefact-unchanged requirement hold by construction.
- **Placement is the main design decision.** ADR-0069 forbids a domain crate
  from depending on a codec. Its rule 1 says a domain crate "reaches a
  technical library only through one of its own ports", and a domain crate's
  closure must be free of codecs. So `pulldown-cmark` must not go into the
  `collaboration` domain crate. The role lint
  (`tasks/lint/crate_dependencies.py`) would also reject a
  `collaboration → technical-library` edge. The rule-consistent shape is:
  - a new technical-library crate that holds the unwrap
  - consumed by an adapter (`cli/github`) or the composition root
    (`collaboration-cli`), or by a new `collaboration` port

  The work item names no host crate. Review 1 also flagged this as
  outstanding.
- **The `SoftBreak` range covers the line ending only.** This was settled
  empirically on `pulldown-cmark` 0.13.4. The range is `\n` or `\r\n`. It
  excludes:
  - trailing whitespace before the line ending
  - the next line's `>`/indent prefix

  The replaced span therefore has to be computed. Taking "next event start"
  as its end is wrong: `foo\n\*bar` becomes `foo *bar`, because the `Text`
  event starts after the escape. The rule that works is
  `[start − trailing spaces/tabs, min(next event start, end of a forward scan over '>', ' ', '\t'))`.
  It passes all 30 fixtures (the acceptance criteria plus four from review 1) under both LF and CRLF, along
  with the idempotence and normalised event-stream checks.
- **The supply chain is cheap.**
  - Only `pulldown-cmark` and `pulldown-cmark-escape` would be new packages.
    `bitflags` 2, `memchr` and `unicase` are already in `cli/Cargo.lock`.
  - All the licences are in `cli/deny.toml`'s allow-list.
  - Use `default-features = false` to avoid `getopts`.
  - ⚠️ Pin it exactly (`=0.13.x`). 0296's sync digest requires an
    exact-version parser and would otherwise tighten it later.
- **The skill wording is narrower than the work item assumes.**
  `describe-pr` SKILL.md never says "exactly" or "byte-for-byte". The only
  claim is at `skills/github/describe-pr/SKILL.md:145-146`: "(without
  frontmatter — they'll see what gets posted to GitHub)". The reference copy
  under `docs-site/.../reference/skills/` is generated and gitignored. The
  hand-written `docs-site/src/content/docs/collaboration.md:34-38` describes
  `update-body` and needs a sentence.

## Detailed Findings

### The `update-body` path today

```text
describe-pr Step 9 ── strips frontmatter ──▶ {tmp}/pr-body-{n}.md
        │
        ▼
collaboration-cli  run_update_body  (main.rs:215-251)
        │ read_to_string → String (UTF-8 required, exit 2 on failure)
        ▼
collaboration  update_pull_request_body(…, body: &str)  (update_body.rs:31-60)
        │ resolve_base_repository → body_update.update_body(owner, repo, n, body)
        ▼
github  OctocrabClient::update_body  (octocrab_client.rs:136-151)
        │ pulls().update(n).body(body).send()
        ▼
PATCH /repos/{owner}/{repo}/pulls/{n}   {"body": …}
```

- **Clap surface.** `cli/collaboration-cli/src/cli.rs:30-35` declares
  `UpdateBody { pull_number: u64, #[arg(long)] body_file: PathBuf }`. So
  `--help` lists `<PULL_NUMBER>`, `--body-file <BODY_FILE>` and `-h`. The
  "help unchanged" criterion holds as long as no flag is added.
- **File read.** `cli/collaboration-cli/src/main.rs:219-224` reads the file
  with `std::fs::read_to_string`. Failure raises `kernel::Error::Refusal`,
  which exits 2. The read happens before any config or network access, and
  the file is never written.
- **Domain function.** It is at `cli/collaboration/src/update_body.rs:31-60`.
  It returns `UpdateBodyOutcome::{Updated, BaseRepoResolutionFailed, PatchFailed}`.
  There is no body newtype.
- **Port.** `PullRequestBodyUpdate::update_body(&self, owner, repo,
  pull_number, body: &str)` is at `cli/collaboration/src/lib.rs:106-117`.
- **Adapter.** `cli/github/src/octocrab_client.rs:136-151` is the only code
  that builds the request. It has role `adapter` and context
  `collaboration`.
- **Crate shape.** `cli/collaboration/Cargo.toml` depends only on `kernel`,
  and its metadata is `role = "domain"`, `context = "collaboration"`,
  `kind = "product"`. It has no dev-dependencies.

### Test surfaces for the acceptance criteria

- **End-to-end tests** live in `cli/collaboration-cli/tests/end_to_end.rs`:
  - `update_body_succeeds` (185-233) registers the PATCH route and asserts
    exit 0 with empty stdout. It never inspects the payload.
  - `scratch_repo()` (18-38) and `run()` (61-78) are the harness.
  - `run()` points the binary at the mock through
    `ACCELERATOR_COLLABORATION_GITHUB_API_URL`.
- **`MockHTTPServer::last_body(&RequestKey) -> Option<Vec<u8>>`** is at
  `cli/http-test-support/src/lib.rs:234-239`. It returns the raw bytes of
  the last request for an exact method and path. Parse them as JSON and
  assert `["body"]` for the first acceptance criterion.
  - ⚠️ It reads the body by `Content-Length` only (376-383), so a chunked
    request would record as empty. Octocrab currently sends
    `Content-Length`; the existing 422 test proves that the request reaches
    the mock, but nothing asserts on its body today.
- **Domain unit tests** are at `update_body.rs:62-253`. They use plain
  `#[test]`, hand-written `Fixed*` stubs and behavioural snake_case names.
  `FixedBodyUpdate` (148-160) ignores its arguments. A recording stub would
  be needed only if the unwrap is applied through a domain port.
- **Adapter tests** are at `cli/github/tests/octocrab_client.rs:184-227`.
  They use `#[tokio::test]` against `OctocrabClient::with_base_uri`. This is
  the natural test site if the adapter applies the unwrap.

### `pulldown-cmark` soft-break behaviour (empirical)

A scratch crate was built offline against `pulldown-cmark` 0.13.4, the
version already in the local cargo registry. It used `default-features =
false` and the five options from the work item. Observed `into_offset_iter`
ranges:

| Input | `SoftBreak` range | Next inline event starts at |
|---|---|---|
| `foo \nbar` | `4..5` (`\n` only; space at 3 orphaned) | `5` |
| `> first line\n> second line` | `12..13` | `15` (after `> `) |
| `> - item\n>   continued` | `8..9` | `13` |
| `foo\n    > bar` | `3..4` | `8` (`Text("> bar")`) |
| `> a  \n>   b\r\n>\t c` | `11..13` (`\r\n`) | `16` |
| `foo\n\*bar` | `3..4` | `5` (after the `\`) |

These observations follow from the table:

- **Prefix exclusion.** The `SoftBreak` range never includes the container
  prefix. The gap between the break and the next event is the `>` markers
  plus the indentation.
- **Gap vs. scan.** The gap is not always prefix-only. An escape at the line
  start (`\*`) or literal text beginning with `>` after deep indentation
  (`foo\n    > bar`) breaks that assumption. Neither "next event start" nor
  "scan over `>`/whitespace" alone is correct. Their minimum is correct.
- **Trailing whitespace.** It belongs to neither the preceding `Text` nor
  the `SoftBreak`. A backward scan over spaces and tabs from the break start
  recovers it.
- **Constructs with no soft-break event.** The following emit no
  `SoftBreak`, so the rewrite leaves them alone without special-casing:
  - multi-line code spans: a single `Code` event spans the newline
  - inline HTML: an `InlineHtml` event contains the newline
  - hard breaks: `HardBreak` ranges include the `  \n` / `\\\n`
  - the alert `[!NOTE]` line: no event; `BlockQuote(Some(Note))`
  - the setext underline
  - link reference definitions
  - indented and fenced code
  - HTML blocks
- **Interruption.** `foo\n- bar`, `foo\n1. bar`, `foo\n# bar` and
  `foo\n> bar` start new blocks. `foo\n2. bar`, `foo\n-bar`, `foo\n#5 bar`
  and `foo\n    > bar` are paragraph continuations. All of this matches the
  work item.
- **Tables.** `Some\nwrapped text\n| a | b |\n| - | - |` parses as a
  paragraph ending at byte 18 followed by a table. Whether `cmark-gfm`
  agrees is the by-hand check the work item already schedules.

These fixtures all pass with the `min` rule:

- every acceptance-criteria fixture
- `foo\n\*bar`
- the review's four extra fixtures (`foo\n   bar`, `[a\nb](u)`,
  `~~a\nb~~`, `> a\n  b`)
- a block of indented code, an HTML block and a multi-line link reference
  definition

Each one gives the expected output, is idempotent, and has an equal
normalised event stream (soft break read as a space, adjacent `Text`
merged), under both LF and CRLF. Probe source:
`scratchpad/probe/src/main.rs` in the research session. It is not committed.

### Architecture gates for adding the parser

ADR-0069 (`meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`)
constrains placement:

- **Rule 1** (96-98): a domain crate depends only on `kernel` and upstream
  domains, and it reaches a technical library through its own port.
- **Technical library** (78): "codecs and I/O primitives belonging to no
  context". The existing ones are `document`, `store`, `process-probe` and
  `remote-projection`.
- **Domain closure** (160-161): it must be free of codecs.
- **Rule 4** (106-109): adapters may depend on technical libraries.
- **Product context** (91, 99): `collaboration` is a product context, so no
  other context may depend on it. An unwrap placed there could never be
  reused by 0320 or 0296.

The role lint enforces only edges between workspace members
(`tasks/lint/crate_dependencies.py:319-329`). A third-party crate added
straight to `collaboration` would therefore not be caught mechanically, even
though it violates the ADR's intent.

| Placement | Gates triggered | Fit with ADR-0069 | Reuse by 0320/0296 |
|---|---|---|---|
| Module in `collaboration` (domain) | snapshot, deny, notices | violates rule 1 / codec-free closure | no (product context) |
| New tech-lib crate, used by `cli/github` adapter | six-point library checklist + deny, notices | fits rule 4 | yes |
| New tech-lib crate, used by `collaboration-cli` before the domain call | same | fits (composition root) | yes |
| New tech-lib crate behind a new `collaboration` port | same + `collaboration` snapshot change | fits rule 1 | yes |

**Adding a technical-library crate** costs the six obligations in
`tasks/README.md:790-854`:

1. Workspace membership, then sync the lock via `cargo metadata`, never
   `generate-lockfile`.
2. Inherited manifest fields and `[lints] workspace = true`.
3. `[package.metadata.accelerator] role = "technical-library"`, plus a row
   in `_EXPECTED_DECLARATIONS` (`tests/unit/tasks/test_crate_dependencies.py`).
4. A `cli/pup.ron` rule.
   - Precedent: `design_domain_imports_only_permitted` permits named pure
     crates (`pup.ron:534-557`).
   - ⚠️ Grouped `use a::{b, c}` resolves to an empty module name under
     `allowed_only` (`pup.ron:246-249`).
5. A violation and control probe pair in
   `tests/integration/pup/test_import_rule.py`.
6. Classification in `tasks/public_api.py`.
   - Technical libraries are pinned in practice: `document`,
     `remote-projection` and `store` are in `_PINNED_CRATES`
     (`public_api.py:20-25`); only `process-probe` is exempt (84).
   - If the crate is pinned, keep `pulldown_cmark` types out of its public
     signatures. `tasks/README.md:834-852` prefers removing that exposure
     to accepting a snapshot that drifts with every parser bump.

**Changes needed in any placement:**

- **`[workspace.dependencies]`** (`cli/Cargo.toml:56-228`): add
  `pulldown-cmark` there with a rationale comment and inherit it with
  `workspace = true`.
  - Exact pins are the house style where output is golden-tested, e.g.
    `similar = "=2.7.0"` at 92-96.
- **`cli/deny.toml`.**
  - Licences: MIT and MIT/Apache are already allowed.
  - Duplicates: `multiple-versions = "deny"` (117). `bitflags` 2.13.1 is
    already the 2.x line, so no new skip entry is expected. I did not run
    `deny:check` to confirm.
- **`licenses/accelerator-third-party-notices.txt`** drifts, because the
  crate links into the shipped `accelerator-collaboration` binary. Run
  `mise run notices:update`.
- **MSRV.** The workspace has `rust-version = "1.90.0"` with
  `resolver = "3"`. 0.13.4 declares `rust-version = "1.71.1"`, which is well
  inside that.

### Existing Markdown handling in `cli/`

There is no Markdown parser in the workspace. The existing handling is all
line-based:

- frontmatter splitting: `cli/document/src/fence.rs`
- section diffing: `cli/work/src/section_diff.rs`, which is not fence-aware
- a hand-built Markdown↔ADF converter: `cli/jira-client/src/adf/`. Its
  module doc says no crate reproduces Jira's dialect.

None of these is reusable for the unwrap. The new crate would be the first
CommonMark parser in the closure.

### `describe-pr` and documentation

- **`skills/github/describe-pr/SKILL.md`.**
  - 145-146 is the only claim that needs rewording: "Show the user the
    generated description (without frontmatter — they'll see what gets
    posted to GitHub)."
  - Step 9 (157-185) strips the frontmatter into
    `{tmp directory}/pr-body-{number}.md` and calls `update-body` (174).
    That is a natural place for the "soft line breaks are unwrapped before
    posting" sentence.
- ⚠️ **Existing drift at SKILL.md 181-182.** It lists "body-file read
  failure" under exit 1. The code exits 2 (`Refusal`), as
  `docs-site/src/content/docs/collaboration.md:36` correctly says. This is
  worth fixing while editing.
- **`docs-site/src/content/docs/reference/skills/github/describe-pr.md`**
  is generated (`DOCS_GENERATED_DIR`, `tasks/shared/paths.py:14`) and
  gitignored (`.gitignore:28`). Do not edit it.
- **`docs-site/src/content/docs/collaboration.md:34-38`** describes
  `update-body`. It is hand-written and should gain the unwrap behaviour.
- **Wording tests.** No test asserts on SKILL.md wording.
  `test_conformance.py`, `test_skill_frontmatter_*.py` and `skills-schema.tsv`
  only list the path. The fourth acceptance criterion has no automated
  guard.
- **Callers of `update-body`.** The only callers are `describe-pr` (allowed
  tools at :10, invocation at :174) and `.claude/skills/raise-pr/SKILL.md:124`
  through `describe-pr`. This confirms the work item's claim.

## Code References

- `cli/collaboration-cli/src/cli.rs:30-35` – `UpdateBody` clap declaration
- `cli/collaboration-cli/src/main.rs:215-251` – `run_update_body`; file read
  at 219-224, domain call at 233-242
- `cli/collaboration-cli/src/main.rs:60-106` – `BlockingGitHubClient`
  (`update_body` at 91-106)
- `cli/collaboration/src/update_body.rs:13-60` – `UpdateBodyOutcome`,
  `update_pull_request_body`
- `cli/collaboration/src/update_body.rs:62-253` – unit tests, `Fixed*` stubs
- `cli/collaboration/src/lib.rs:106-117` – `PullRequestBodyUpdate` port
- `cli/collaboration/tests/fixtures/public-api.txt:28-41,79` – pinned
  public-API snapshot
- `cli/github/src/octocrab_client.rs:136-151` – PATCH via octocrab
- `cli/github/tests/octocrab_client.rs:184-227` – adapter tests
- `cli/collaboration-cli/tests/end_to_end.rs:185-333` – update-body e2e tests
- `cli/http-test-support/src/lib.rs:234-239` – `MockHTTPServer::last_body`
- `cli/Cargo.toml:1-54,56-228,230-244` – workspace, deps convention, lints
- `cli/deny.toml:44-65,116-178` – licences, bans, duplicate skips
- `cli/pup.ron:246-249,534-557` – grouped-import gotcha; pure-crate
  permit-list precedent
- `tasks/public_api.py:15-30,46-94,101-147` – pinned/exempt crates, snapshot
  check
- `tasks/lint/crate_dependencies.py:72-74,319-329,378-389` – role rules
- `tasks/README.md:790-854` – library-crate checklist
- `skills/github/describe-pr/SKILL.md:145-146,157-185` – shown-vs-posted
  claim, Step 9
- `docs-site/src/content/docs/collaboration.md:34-38` – `update-body` docs

## Architecture Insights

- **Use-case modules.** Each use case is one module with one outcome enum and
  one entry function (`base_repo`, `update_body`). The shared ports and
  vocabulary live in `lib.rs`.
- **Fakes and test names.** Tests use hand-written fakes, never a mocking
  framework, and behavioural test names. Expect the unwrap's tests to be
  plain `#[test]` fixtures in the same style.
- **Domain sentence.** "Unwrapped bodies are a property of pushing to
  GitHub" (0332 Drafting Notes). That reads most naturally as adapter
  knowledge about GitHub's renderer, which argues for applying the unwrap in
  or beside the `cli/github` adapter rather than in the domain. The
  counter-argument is that a domain port makes the rule visible in the use
  case.
- **Pinning.** Third-party deps carry written rationale and an explicit pin
  policy. A parser whose events feed golden fixtures and 0296's digest fits
  the exact-pin precedent.

## Historical Context

- `meta/work/0071-describe-pr-base-repo-resolver-uses-unsupported-gh-field.md:84,92`
  – origin of the byte-for-byte body contract.
  - Its review (`meta/reviews/work/0071-…-review-1.md:250`) already noted
    that GitHub's line-ending normalisation risks false negatives. This is
    relevant to 0332's CRLF criteria and to the `POST /markdown` check.
- `meta/work/0197-accelerator-collaboration-pr-helper-cli.md:133-136` and
  `meta/plans/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`:
  - the payload is carried forward "unchanged" (133-136)
  - the domain crate is kernel-only (553-556)
  - file I/O is kept in the CLI (724-727)
- `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker.md`:
  - It uses the same normalised event-stream equivalence (78-88).
  - It requires an exact parser pin (237-239, 353-358).
  - It leaves `pulldown-cmark` vs `comrak` open (415-416).

  0332 effectively decides the parser for it.
- `meta/work/0320-linear-push-corrupts-markdown-content.md`: the unwrap
  fixes defect 1 only.
  - Defect 2 (autolinked dotted tokens) needs escaping.
  - Defect 3 (wrapped code span) needs code-span newlines collapsed, which
    0332 explicitly preserves.

  The `relates_to` link is one-way (0320 lists only 0296).
- `meta/reviews/work/0332-push-pr-descriptions-to-github-unwrapped-review-1.md`
  – pass 4 (462-504), verdict REVISE. Its three majors appear resolved in
  the current text. These minors remain:
  - fixtures for `foo\n   bar`, `[a\nb](u)`, `~~a\nb~~`, `> a\n  b`
  - literal footnote and interleaved fixtures
  - no host crate named
  - one-way 0320 link
  - unbounded corpus fix loop
  - 0071 and 0152 absent from Dependencies

## Related Research

- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`
- `meta/research/codebase/2026-05-18-0071-describe-pr-base-repo-resolver-unsupported-gh-field.md`
- `meta/research/codebase/2026-10-03-cli-surface-inventory.md`
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`

## Open Questions

- ❓ **Host crate and application point.** The options are a new
  technical-library crate (a name such as `markdown`) applied in:
  - the `cli/github` adapter
  - `collaboration-cli`
  - a new `collaboration` port

  The adapter fits the "property of pushing to GitHub" framing and ADR-0069
  rule 4. A port makes the rule part of the domain use case at the cost of a
  `collaboration` snapshot change.
- ❓ **Exact pin version.** The latest local version is 0.13.4. The work item
  says "0.11.0 or later". 0296 needs an exact pin, so decide whether 0332
  sets it now.
- **Not verified here:** `deny:check` with the new dependency.
- **Cross-cutting precedence.** The probe's span rule is an observed
  behaviour of 0.13.4, not a documented contract. Pin it with the work
  item's "test first" fixture before relying on it, and re-run on any parser
  bump.
