---
type: "codebase-research"
id: "2026-10-09-0225-scheduled-advisory-and-age-guards"
title: "Codebase surface for the scheduled advisory and age guards on the vendored runtime pins"
date: "2026-10-09T18:09:02+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0225"
parent: "work-item:0225"
topic: "Codebase surface for the scheduled advisory and age guards on the vendored runtime pins"
tags: ["research", "codebase", "tasks", "vendor", "pins", "keyring", "gpg", "github-actions", "advisories", "runtime-pin-guard"]
revision: "d64bc1dbfef902d1c0a09fac9f993c303fbe9dc3"
repository: "accelerator"
last_updated: "2026-10-09T20:37:20+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research computing the initial bump dates"
schema_version: 1
---

# Codebase surface for the scheduled advisory and age guards on the vendored runtime pins

**Date**: 2026-10-09 19:09 BST
**Author**: Toby Clemson
**Git Commit**: `d64bc1dbfef902d1c0a09fac9f993c303fbe9dc3` (jj change `qmwrutrxynzl`)
**Branch**: none (detached working-copy change in the `build-system` workspace)
**Repository**: accelerator

## Research Question

What does the codebase already provide, and what is missing, for work item
0225: a scheduled `mise` task and GitHub workflow that matches the three
vendored runtime pins (`playwright-core`, Node, Chromium) against advisory
feeds, checks pin age, keyring age and Node release-key expiry, and opens
deduplicated GitHub issues?

## Summary

None of the guard exists. The relevant pieces are all in `tasks/`, so the
work fits the build-system Python toolchain the work item targets. What
is there:

- **Pins.** `pins.toml` holds `chromium.revision = "1193"` and
  `node.version = "22.22.2"`. The `playwright` version (`1.55.1`) is in
  `skills/design/inventory-design/scripts/playwright/package.json` and is
  read by `assemble.pinned_playwright_version`.
- **Bump dates.** No `bumped` keys, `[playwright-core]` table or
  `[keyring]` table exist yet. Every reader ignores new keys in `[chromium]`
  and `[node]` and ignores new top-level tables. A `bumped` key inside
  `[assembled_sha256.*]` or `[chromium.sha256]` would break the drift test,
  `build.rs`, or `vendor:check-trust-anchors`.
- **HTTP.** `tasks/shared/vendor/fetch.py` is the only HTTP client: httpx,
  GET-only, with injectable `Fetcher` and `JsonFetcher` type aliases. OSV's
  `querybatch` needs a POST, which no port provides today.
- **`browsers.json`.** The only reader is `assemble.browser_revision`, and
  it reads `revision`, not `browserVersion`. Nothing in `tasks/` reads
  `browserVersion`.
- **gpg.** `tasks/shared/vendor/gpg.py` uses gpg for signature verification
  only: an ephemeral homedir, `--status-fd`, and a pure classifier behind an
  injected `Runner`. Nothing lists keys or parses `--with-colons` output.
  gpg is not provisioned by `mise`; the code probes it with
  `shutil.which`.
- **`gh`.** It is pinned in `mise.toml:7` (`2.89.0`). Every existing use is
  `gh release *` or `gh auth status`. There is no `gh issue` or `gh label`
  wrapper.
- **CI.** `.github/workflows/main.yml` is the only workflow. There is no
  `CODEOWNERS`. Invariants for workflows are tested in
  `tests/unit/tasks/test_workflows.py`.
- **`RELEASING.md`.** Its "Vendored-runtime trust anchors" section
  (`:159-216`) lists the anchors and a refresh procedure. It has no owner,
  no maximum ages and no feeds.
- **Closest template.** `docs.audit_check` (`tasks/docs.py:42`) together
  with `tasks/shared/npm_audit.py` already models "advisories plus dated
  lapses, given `today`". It takes `today` as a task argument, keeps the
  domain logic in frozen dataclasses, and fails via
  `raise Exit(..., code=1)`.

## Detailed Findings

### Pins and their readers

`pins.toml` sits at the repository root and is addressed as `PINS_TOML`
(`tasks/shared/paths.py:54`). It contains:

- `[assembled_sha256.browser]` and `[assembled_sha256.driver]`, four
  platform digests each (`pins.toml:11-21`);
- `[chromium] revision = "1193"` (`pins.toml:28-29`);
- `[chromium.sha256]`, four per-platform digests (`pins.toml:31-35`);
- `[node] version = "22.22.2"` (`pins.toml:37-38`). This matches the
  `node` tool pin in `mise.toml`.

Its four readers all look fields up by key:

| Reader | Parser | What it reads | Behaviour on new keys |
|---|---|---|---|
| `tasks/shared/vendor/pins.py` | `tomllib`, re-read per call | `chromium.revision`, `chromium.sha256[platform]`, `node.version`, `assembled_sha256` (`:18-57`) | Ignores them; a missing key raises `KeyError` |
| `tasks/shared/vendor/trust_anchors.py` | `tomllib`, defensive `.get` | Every leaf under `assembled_sha256.*` and `chromium.sha256` is checked as a digest (`:42-74`) | A non-digest leaf in those tables is reported as a placeholder |
| `cli/launcher/build.rs` | `toml` crate | Only `[assembled_sha256]`, strictly (`:57-101`) | A non-table entry or non-string leaf fails the build |
| `tests/unit/tasks/test_tree_artifact_pins.py` | `tomllib` | Only `assembled_sha256` (`:25-69`) | Breaks if the artifact or platform key sets change |

Where `bumped` can safely go:

- `[chromium]` (`chromium.bumped`) and `[node]` (`node.bumped`), as the
  work item specifies;
- new `[playwright-core]` and `[keyring]` tables.

It must never go under `[assembled_sha256]` or `[chromium.sha256]`.
`tomllib` parses an unquoted TOML local date to `datetime.date`. The
existing `str()` coercions in `pins.py` never touch the `bumped` keys.

The `playwright` version comes from `assemble.pinned_playwright_version`
(`tasks/shared/vendor/assemble.py:99-112`). It requires an exact `X.Y.Z`
via `_EXACT_VERSION` (`:51`) and raises `ValueError` otherwise. That
already covers the work item's "unparseable declared `playwright` version"
abort. The `package.json` path is the constant `PLAYWRIGHT_PACKAGE_JSON`
(`tasks/vendor/commands.py:27-29`).

`vendor:check-trust-anchors` (`mise.toml:284-286`,
`tasks/vendor/commands.py:32-39`) only detects placeholders. It runs on
push in `assemble-runtime` (`.github/workflows/main.yml:469-473`).

### Upstream fetch and the `playwright-core` tarball

`upstream.verify_upstream_inputs` (`tasks/shared/vendor/upstream.py:82-151`)
is the existing orchestrator with injected ports: `fetch: Fetcher`,
`fetch_json: JsonFetcher`, `slsa_runner` and `gpg_runner`, all defaulting
to the real implementations. The path relevant to 0225:

1. `npm_packument_url("playwright-core")` builds
   `https://registry.npmjs.org/playwright-core` (`:24`, `:61-79`).
2. `npm.packument_dist(packument, version)`
   (`tasks/shared/vendor/npm.py:57-78`) returns a `DistInfo` with
   `tarball`, `integrity`, `signature_b64` and `attestations_url`. It raises
   `ValueError` if the version is absent or the dist has no signature or
   attestations.
3. `fetch(dist.tarball, staging/...)` downloads the tarball (`:100-103`).
   The tarball is never opened at fetch time. Extraction happens later in
   `assemble.assemble_tree_artifacts`, using `tarfile` with
   `filter="data"` (`assemble.py:333-396`).

How `browsers.json` is used today:

- `assemble.browser_revision(browsers_json, name)` (`assemble.py:115-126`)
  iterates `document["browsers"]`, matches `name` exactly, and returns
  `str(entry["revision"])`. It never reads `browserVersion`.
- `chromium.verify_chromium` (`chromium.py:48-63`) and
  `assemble.assert_version_pairing` (`assemble.py:129-152`) cross-check the
  revision against `pins.toml`. ⚠️ Both have no production callers: only
  their tests call them. The docstring at `chromium.py:36-38` says the
  cross-check "happens at assembly", but nothing calls it there. So
  0225's pin-inconsistency abort would be the first production code to
  check `chromium.revision` against the pinned `playwright-core`'s
  `browsers.json`.

The HTTP layer is `tasks/shared/vendor/fetch.py`:

- `get_json(url)` is `httpx.get(..., timeout=60, follow_redirects=True)`,
  then `raise_for_status()`, then `.json()` (`:38-42`).
- `download(url, dest)` streams with a 300 s timeout (`:26-35`).
- Errors are not wrapped: `httpx.HTTPStatusError`,
  `httpx.TransportError` and `json.JSONDecodeError` reach the caller
  unchanged. 0225 must map each of these, plus missing fields, to a feed
  failure.
- There is no POST port, so OSV `querybatch` needs a new one. KEV,
  `vuln/core` (`raw.githubusercontent.com`) and `GET /v1/vulns/{id}` are
  all plain GETs.
- httpx is exact-pinned in `pyproject.toml:26-29`.

### Keyring and OpenPGP tooling

The keyring has three files:

- `keys/nodejs-release.asc`: one armoured block of about 467 lines
  concatenating the Node release keys. The first is an ed25519 key.
- `keys/npm-registry.pem`: a 4-line EC P-256 SubjectPublicKeyInfo.
- `keys/accelerator-release.pub`: the project's minisign key. It is not
  part of the 0225 keyring.

`nodejs.NODE_RELEASE_FINGERPRINTS` (`tasks/shared/vendor/nodejs.py:29-39`)
allowlists 9 primary fingerprints. ⚠️ The comment at `:24-26` cites "a
build-system consistency test" against the committed keyring. No such test
exists; the only keyring tests check that the file exists.

How gpg is used today (`tasks/shared/vendor/gpg.py`):

- `_run_gpg` (`:135-175`) returns `None` when `shutil.which("gpg")` is
  absent. Otherwise it creates a `TemporaryDirectory` as `--homedir`,
  imports the armoured keyring with `--batch --quiet --import` (the
  result is ignored), and runs `--status-fd 1 --verify`.
- `classify_status_lines` (`:54-107`) is the pure predicate.
  `type Runner = Callable[[Path, Path, Path], list[str] | None]` (`:112`)
  is the seam.
- `EXPKEYSIG` is deliberately accepted (`:74-79`). An expired Node signer
  therefore does not break releases. Key expiry is a release-time non-event
  that only a scheduled guard surfaces, which is consistent with 0225's
  issue-not-failure design.
- Nothing calls `--list-keys`, `--show-keys` or `--with-colons`, and
  nothing reads key capabilities or expiry.

Provisioning:

- gpg is not in `mise.toml [tools]` and no workflow step installs it. CI
  relies on it being on `ubuntu-latest`.
- cosign (`mise.toml:39`) and minisign (`:35`) are pinned via `ubi:`.
- The 0196 plan's fallback for unpinned gpg was a behavioural preflight
  (plan L2447-2455).

### Task, port and test patterns to model on

**Closest analogue: `docs.audit_check`** (`tasks/docs.py:42-83`), with the
domain module `tasks/shared/npm_audit.py`:

- `today: str | None` is parsed with `dt.date.fromisoformat`, defaulting to
  `dt.datetime.now(tz=dt.UTC).date()`. This satisfies 0225's "takes the
  current date as an input".
- Frozen dataclasses (`Advisory`, `AdvisoryIgnore`) carry behaviour that
  takes `today: dt.date`.
- A domain `IgnoreListError` is translated to `Exit(..., code=1)` at the
  task boundary.
- Tests in `tests/unit/tasks/test_docs_audit_check.py` fix
  `TODAY = "2026-10-03"`, stub `MagicMock(spec=Context)` with canned JSON,
  and assert with `pytest.raises(Exit, match=...)`.

**Port styles.** The repository has three. The injected-port style fits
the work item's in-memory fake issue tracker.

| Style | Example | Test approach |
|---|---|---|
| `context.run` string commands | `tasks/github.py:53-89` (`gh auth status`, `gh release create`) | `MagicMock(spec=Context)`; asserts on the command string |
| Direct `subprocess.run` | `tasks/github.py:98-123` (`gh release download`) | Monkeypatches `subprocess.run` |
| Injected `Protocol` or `Callable` port | `gpg.Runner`; `ProcessOps` (`tasks/shared/processes.py:34-87`); `MeasurementRunner` / `DiagnosticRunner` (`tasks/measure.py:401-563`) | Hand-written fakes, e.g. `FakeProcs` in `tests/unit/tasks/shared/doubles.py:28-74` |

Two further precedents:

- **Composition root.** `tasks/dev.py:51-74` builds a frozen `DevDeps`
  bundle of real ports. Tests swap in fakes, as in
  `tests/unit/tasks/shared/dev/test_lifecycle.py:23-204`.
- **Fixtures.** Recorded responses go under
  `tests/unit/tasks/fixtures/<topic>/` and are loaded via
  `Path(__file__).resolve().parent / "fixtures" / "<topic>"`. The vendor
  tests themselves use inline dicts, not recorded files, despite their
  docstrings. 0225's "one fixture per feed for each of unreachable,
  unparseable and missing-field" fits the fixtures-directory pattern
  better.

**Registration:**

1. Add a `@task` and wire it in `tasks/__init__.py`, either with
   `Collection.from_module` (`:60-72`) or a hand-built `Collection`
   (`:74-80`).
2. Add a `mise.toml` leaf: `run = "invoke <ns>.<task>"` with
   `depends = ["deps:install:python"]`.
3. Update the roll-up guards in `tests/unit/tasks/test_mise.py`.
   `test_mise.py:245-310` enforces that networked, credentialed tasks such
   as `measure:*` stay out of both `check` and `default`. The guard needs
   network and a `gh` token, so it belongs in the same excluded category.
4. Follow `tasks/README.md`'s naming: standalone entity tasks lead with
   the entity (`deny:check`, `github:*`).

**Failure reporting.** The convention is `raise Exit(message, code=1)`,
with domain `…Error` exceptions translated at the boundary. ruff
`select=ALL` enforces the `Error` suffix.

### CI workflow

`.github/workflows/main.yml`:

- **Triggers:** `push` to `main` and `pull_request` only (`:3-12`). There
  is no `schedule` or `workflow_dispatch` anywhere.
- **Permissions and concurrency:** set per job, with no workflow-level
  defaults. Examples: `assemble-runtime` has `permissions: {}` (`:457`);
  the release jobs use the `accelerator-release` concurrency group
  (`:582-585`).
- **Pinning:** actions are SHA-pinned. `jdx/mise-action` runs with
  `install: true, cache: true, experimental: true`.
- **`gh` authentication:** `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}` on
  specific steps (`:640-641` etc.). 0225 needs `issues: write` plus
  `GH_TOKEN` on the guard step.
- **Invariant tests:** `tests/unit/tasks/test_workflows.py` holds
  structural invariants, e.g.
  `test_assemble_and_smoke_hold_no_permissions` (`:611`). A new workflow
  file for the guard would get its acceptance-criteria invariants
  (`schedule`, `workflow_dispatch`, `issues: write`, `concurrency`) tested
  the same way. I did not check whether that test module is hard-wired to
  `main.yml`.

### `RELEASING.md`

The relevant structure:

- `## Vendored-runtime trust anchors` (`:159`) has an anchor table
  (`:167-173`) and the placeholder guard (`:175-180`).
- `### Refreshing the anchors` (`:182-216`) is a four-step refresh: keys
  verified out of band, pins set, local assembly, then the guard passes.
  This is the natural place for "update the bump date in the same change".

What it lacks:

- **Owner.** Only "release-owner team" (`:360`), "an admin" (`:96`) and
  "the release admin" (`:146`). There is no `Owner: <name> (@login)`
  line.
- **Maximum ages.** None for pins or keys. Refresh is "on initial setup
  and whenever the pinned Playwright version bumps" (`:184-185`).

It also has existing drift:

- `trust_anchors.py:127-128` points to a "Refreshing the vendored-runtime
  trust anchors" heading. The actual heading is `### Refreshing the
  anchors`.
- `RELEASING.md:164-165` and `trust_anchors.py:3-4` say a fresh checkout
  ships placeholders. The committed values are real.

## Code References

- `pins.toml:28-38` — the Chromium and Node pins; the home of the new
  `bumped` keys
- `tasks/shared/paths.py:54` — `PINS_TOML`
- `tasks/shared/vendor/pins.py:18-57` — pin accessors (`KeyError` on
  missing)
- `tasks/shared/vendor/trust_anchors.py:42-129` — placeholder guard;
  constrains where `bumped` may go
- `cli/launcher/build.rs:57-101` — strict `[assembled_sha256]` reader
- `tests/unit/tasks/test_tree_artifact_pins.py:25-69` — drift test
- `tasks/shared/vendor/assemble.py:51,99-126` — `pinned_playwright_version`,
  `browser_revision`
- `tasks/shared/vendor/chromium.py:25,48-63` — `chromium-headless-shell`
  constant; `verify_chromium` with no production caller
- `tasks/shared/vendor/upstream.py:24-151` — registry URL, packument and
  tarball fetch, injected ports
- `tasks/shared/vendor/npm.py:47-78` — `DistInfo`, `packument_dist`
- `tasks/shared/vendor/fetch.py:18-42` — `Fetcher`, `JsonFetcher`,
  `get_json`, `download`
- `tasks/shared/vendor/gpg.py:31-175` — `Verdict`, `classify_status_lines`,
  `Runner`, `_run_gpg`
- `tasks/shared/vendor/nodejs.py:24-39` — `NODE_RELEASE_FINGERPRINTS` and
  the stale consistency-test comment
- `tasks/docs.py:42-83` — `audit_check`, the task template with a `today`
  argument
- `tasks/shared/npm_audit.py:1-96` — domain template: advisories, dated
  lapses
- `tasks/github.py:53-123` — existing `gh` invocations
- `tasks/shared/processes.py:34-87`,
  `tests/unit/tasks/shared/doubles.py:28-74` — Protocol port plus
  in-memory fake
- `tasks/__init__.py:60-80` — collection wiring
- `mise.toml:7,147-150,284-298` — `gh` pin, the `docs:audit:check` leaf,
  the vendor leaves
- `tests/unit/tasks/test_mise.py:245-310` — guard keeping networked tasks
  out of `check`/`default`
- `tests/unit/tasks/test_workflows.py:611-628` — workflow invariant tests
- `.github/workflows/main.yml:453-507` — `assemble-runtime`, the only
  current consumer of the keyring and gpg
- `RELEASING.md:159-216` — trust-anchor section and refresh procedure

## Architecture Insights

The guard decomposes naturally along the existing seams:

```text
                    +-------------------------+
   pins.toml ------>|  local inputs (abort)   |<---- RELEASING.md Owner:
   package.json --->|  pins, bump dates, owner|<---- keys/nodejs-release.asc
                    +-----------+-------------+
                                | evaluated first
                                v
  JsonFetcher / POST port  +---------+   key-lister port (gpg --with-colons)
  OSV, vuln/core, KEV ---->| checks  |<---------------------------------
  npm Fetcher ------------>| (pure)  |
                           +----+----+
                                | findings + feed failures
                                v
                      +-------------------+     gh issue/label port
                      | issue reconciler  |---> (fake in tests)
                      +-------------------+
                                | exit 0 / Exit(code=1)
                                v
                          @task (thin)
```

Design consequences:

- **Domain logic stays pure.** Version matching (`vuln/core` semver ranges,
  four-part Chromium comparison, OSV fixed-version selection), age and
  expiry arithmetic, finding identity and dedup markers can all be plain
  functions over frozen dataclasses with `today: dt.date`, following
  `npm_audit.py` and `gpg.classify_status_lines`.
- **Ports follow existing shapes.** HTTP extends `fetch.py`. The issue
  tracker is a new `Protocol` with a `subprocess`-backed `gh` adapter and
  an in-memory fake. Key listing is a `Runner`-style callable returning
  colon-format lines, so the parser can be table-tested over recorded
  output, as gpg status lines already are.
- **Feed failure needs an error taxonomy at the port boundary.** `fetch.py`
  does not wrap anything, and `npm.packument_dist` raises bare
  `ValueError`. The guard must catch httpx errors, JSON decode errors and
  missing fields per request and attribute each to its feed. It must not
  let these escape as an abort, which is reserved for local inputs.
- **The `playwright-core` tarball path can reuse `packument_dist` and
  `download`.** It then needs a new reader for `package/browsers.json`,
  one that keeps `browserVersion` and distinguishes an empty `browsers`
  array (feed failure) from a populated array lacking the revision (abort).
  The work item does not require signature verification on this fetch.
  Reusing `verify_registry_signature` would be cheap.
- **Placement in the task tree.** The guard is networked and credentialed,
  like `measure:*`, so it stays out of `check` and `default`, with the
  `test_mise.py` guards updated to match.

## Historical Context

- `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`:
  - L2502-2512, point 4: designs the scheduled expiry and age guard as
    issue-opening rather than a unit test.
  - L2438-2445: derives the keyring maximum age from GnuPG reporting
    `REVKEYSIG` only for locally present revocations.
  - L3202-3203: the unticked criterion for this guard.
  - L294-299: the later `EXPKEYSIG`-accepted policy change.
  - L2447-2455: the behavioural-preflight fallback for unpinned gpg.
- The plan's second-approver gate (L2475-2497) was never built
  (L316-323). `prs/72-description.md` (~L82) records it as deferred. No
  work item tracks it yet.
- `meta/reviews/work/0225-scheduled-advisory-and-age-guards-for-the-vendored-runtime-pins-review-1.md`:
  six passes, final verdict COMMENT. It deferred to planning:
  - the OpenPGP tooling;
  - the task name;
  - label and repository setup;
  - a work item for the approver gate;
  - an annotation on the 0196 plan recording that the scheduled guard was
    never built.
- `meta/decisions/ADR-0059-build-time-assembly-of-vendored-browser-artifacts.md`:
  Chromium is pinned by sha256, not provenance-verified, which is why
  advisory matching is the only vulnerability signal for it.
  `ADR-0050-mise-invoke-task-runner.md` covers the task-runner choice.
- `meta/plans/2026-07-02-0162-rust-toolchain-guard-rails.md` and its
  research (`meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md`):
  the cargo-deny advisory precedent on the Rust side.

## Related Research

- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- `meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md`
- `meta/research/codebase/2026-08-31-0203-third-party-attribution-artefact.md`

## Open Questions

- ❓ **OpenPGP parser.**
  - The codebase favours `gpg --show-keys --with-colons
    keys/nodejs-release.asc`, possibly via the existing ephemeral-homedir
    pattern. It needs a pure parser for `pub`/`sub`/`fpr` records: field 7
    for expiry, field 12 for capabilities. Signing-capable means `s` in
    capabilities. Encryption-only subkeys carry only `e`.
  - I have not verified the colon-field semantics against the gpg version
    on `ubuntu-latest`, nor how `--show-keys` reports already-expired
    subkeys.
  - gpg stays unpinned either way, so the plan needs the probe or
    preflight the work item asks for.
- ❓ **Task name and namespace.** `vendor:*` is the existing home for
  runtime-pin tasks. A standalone entity name such as
  `runtime-pin-guard:run` would match the issue label.
- **Initial bump dates.** Computed and decided in the follow-up below.
- ❓ **Pre-existing drift.** Should the plan fold in the fixes alongside
  the `RELEASING.md` edits? The items are:
  - the stale heading reference in `trust_anchors.py:127-128`;
  - the "fresh checkout ships placeholders" text;
  - the missing fingerprint-consistency test cited in `nodejs.py:24-26`;
  - the uncalled `verify_chromium` and `assert_version_pairing`.
- ⚠️ **Scheduled behaviour only shows after merge.** A scheduled workflow
  runs only from the default branch. Token, label and assignee behaviour
  is therefore only observable post-merge, which is why the work item
  includes dispatched-run acceptance criteria.

## Follow-up Research 2026-10-09T20:37:20+00:00

### Initial bump dates

These are the recommended seed values. Ages are as of 2026-10-09. Each
date is the committer date in UTC of the commit on `main`.

| Bump key | Value | Date | Commit | Age (days) |
|---|---|---|---|---|
| `playwright-core.bumped` | `1.55.1` | `2026-05-18` | `7507a77870` | 144 |
| `chromium.bumped` | `1193` | `2026-05-18` | `7507a77870` | 144 |
| `node.bumped` | `22.22.2` | `2026-08-24` | `ccd71e55af` | 46 |
| `keyring.bumped` | Node and npm keys | `2026-08-24` | `dbe0b59d38` | 46 |

With these seeds, the guard's first run opens pin-age issues for
`playwright-core` and Chromium. That outcome is correct, not noise: the
same Playwright 1.55.1 runtime has shipped since May. Node breaches its
maximum age from 2026-11-23 and the keyring from 2027-08-25.

### Method

`main` is linear: every commit in `::trunk()` has one parent, so each
commit's own date is meaningful. I listed every trunk commit touching
`pins.toml`, `tasks/vendor/pins.py`, `tasks/shared/vendor/pins.py`,
`keys/`, the vendored `package.json` and its former `package-lock.json`,
then read the pin values at each of them with `jj file show`. Nothing in
`pins.toml`, `keys/` or the `package.json` changed between `63f1c0e5f7`
and `trunk()` (`cdf7ce04f3`).

### Value history

| Commit | Committer date (UTC) | Change |
|---|---|---|
| `4c328acc6a` | 2026-05-07 | `playwright` `~1.49.0`; lock resolves `playwright-core` 1.49.1 |
| `7507a77870` | 2026-05-18 | `playwright` `~1.55.1`; lock resolves 1.55.1 (closes GHSA-7mvr-c777-76hp; pushed directly, no PR) |
| `32491cbc35` | 2026-08-24 | specifier `~1.55.1` → exact `1.55.1`; resolved version unchanged |
| `703e53e8d4` | 2026-08-24 | `chromium.revision = "0000000"` and `node.version = "0.0.0"` placeholders added |
| `dbe0b59d38` | 2026-08-24 | adds `keys/nodejs-release.asc` and `keys/npm-registry.pem` |
| `ccd71e55af` | 2026-08-24 | placeholders → `revision = "1193"`, `version = "22.22.2"` |
| `ce751dfcf3` | 2026-08-24 | deletes `package-lock.json` |

The 2026-08-24 commits arrived in PR #72, which GitHub records as merged at
2026-08-26T22:57:52Z. Their committer dates are 2026-08-24 (author dates
2026-08-20 and 2026-08-21). I recommend the committer date: it is what
"the date of the default-branch commit" names, and it is reproducible from
VCS alone. The merge date would make Node and the keyring 2 days younger
(44 days), which is immaterial.

### Interpretation choices

- ⚠️ **`playwright-core`: specifier versus version.** Taken literally, the
  value last changed at `32491cbc35` (2026-08-24), giving 46 days. That
  commit only tightened the specifier, though; the resolved runtime
  version, 1.55.1, was fixed by the lockfile at `7507a77870`. Seeding from
  the specifier change would hide an already-stale runtime, which is the
  risk review 1 pass 6 raised. I recommend 2026-05-18.
- ⚠️ **Chromium: placeholder-to-real versus the build in use.** Taken
  literally, `chromium.revision` last changed at `ccd71e55af` (placeholder
  → `1193`, 2026-08-24). Revision 1193 is determined by `playwright-core`
  1.55.1's `browsers.json`, though, so it is the same browser build users
  have run since 2026-05-18. The two pins also move together on every
  future bump. I recommend 2026-05-18 for consistency. The literal reading
  gives 2026-08-24, and the pin-age issue would then open on 2026-11-23
  instead of on the first run.
- **Node.** There was no vendored Node before `ccd71e55af`; the version was
  chosen in that commit. 2026-08-24 is unambiguous.
- **Keyring.** `keys/` also holds `keys/accelerator-release.pub` (touched
  by `f84a46c83d` 2026-07-04 and `575bfd73ee` 2026-07-08). It is not part
  of the 0225 keyring and predates `dbe0b59d38` anyway, so both readings
  give 2026-08-24.

**Decision (2026-10-09):** the May dates are adopted. Work item 0225's Terms
entry for bump date and its `pins.toml` criterion now define the seed as
the commit that last changed the pinned runtime version.

### Not checked

- I did not check whether `chromium-headless-shell` revision 1193 in
  1.55.1's `browsers.json` matches what the 1.55.1 lockfile era installed.
  I inferred it from the pairing and did not fetch the tarball.
- I did not check the GitHub release dates of Playwright 1.55.1 or Node
  22.22.2. Upstream release age is not what the work item measures.
