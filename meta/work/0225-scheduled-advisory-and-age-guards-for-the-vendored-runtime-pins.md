---
type: "work-item"
id: "0225"
title: "Scheduled Advisory and Age Guards for the Vendored Runtime Pins"
date: "2026-08-20T00:00:00+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0196", "work-item:0219"]
tags: ["security", "distribution", "runtime", "playwright", "ci", "advisories"]
last_updated: "2026-10-09T17:29:19+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Revised after review 1 pass 6: empty browsers array is a feed failure, populated-without-revision is a pin-inconsistency abort; checks evaluated before issues open; bump-date format, future dates and initial-value rule; issue-tracker test seam; multi-request feed failures; live-feed confirmation procedure; OSV database_specific and gh dependencies."
schema_version: 1
external_id: "PP-755"
---

# 0225: Scheduled Advisory and Age Guards for the Vendored Runtime Pins

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add one scheduled CI job that watches the vendored runtime — `playwright-core`,
Node and the Chromium build Playwright pins — and the trust-anchor keyring used
to verify it. The job opens an issue when a pin matches a disclosed advisory
(for Chromium, a known-exploited one), a pin or the keyring exceeds its
maximum age, a Node release key nears or passes expiry, or a feed fails. `cargo-deny` covers Rust crates only, so nothing watches the non-Rust
runtime for disclosed vulnerabilities or neglect. The gap matters because the
vendored browser engine ships to every user and is not re-verified on each
launch.

## Context

The vendored-runtime plan
(`plan:2026-08-11-0196-design-vendored-runtime-distribution`) designed the
age-and-expiry half of this guard and deferred advisory-feed matching as a
follow-up; neither was built. Its trust-anchor section specified one scheduled
job that opens an issue for key expiry, the keyring's maximum age and the
pinned runtime's maximum age, with the three runtime pins recorded in
`RELEASING.md` as security-relevant dependencies with a named owner and a
maximum age. The plan names the Chromium pin `CHROMIUM_REVISION`; it shipped
as `chromium.revision` in `pins.toml`. The advisory-matching rules in this
item are specified here for the first time. Today `.github/workflows/main.yml`
is the only workflow and runs on `push`/`pull_request` only, and `RELEASING.md`
names no owner or maximum age.

The two halves are complementary. Age is cheap and bounds neglect, but is a
proxy: a revision can be fresh and vulnerable, or old and fine. Advisory
matching is the real signal, but no single feed covers all three ecosystems,
and Chromium in particular has no feed that matches by version. The exposure
warrants both: a full browser engine ships to every user, and once installed
the launcher reuses it without re-fetching or re-verifying while the pins are
unchanged.

Key expiry is a scheduled guard rather than a unit test deliberately: a unit
test asserting "each key is unexpired" would redden CI on every branch for every
contributor on the day a key expires, blocking unrelated work instead of
notifying an owner.

## Terms

- **Guard** — the `mise` task the workflow runs. One invocation is a **guard
  run**; it performs every **check**. A check is one unit the guard
  evaluates and can skip: advisory matching for one pin (which may draw on
  several feeds), one pin's age, one key's expiry, or the keyring's age.
- **Pins** — the three runtime pins:
  - the `playwright-core` pin, the exact `playwright` version declared in the
    vendored `package.json` (read by `assemble.pinned_playwright_version`);
  - the **Node pin**, `node.version` in `pins.toml`;
  - the **Chromium pin**, `chromium.revision` in `pins.toml`. It is a
    Playwright build number. Its **browser version** is the `browserVersion`
    that the pinned `playwright-core`'s `browsers.json` gives for that
    revision; advisory matching uses the browser version.
- **Keyring** — the trust-anchor publisher keys: `keys/nodejs-release.asc` and
  `keys/npm-registry.pem`. A **keyring refresh** is replacing a key, or
  re-verifying the keyring against an independent channel and finding it
  unchanged.
- **Bump date** — an explicit date in `pins.toml`, one each for the three
  pins and the keyring (`playwright-core.bumped`, `node.bumped`,
  `chromium.bumped`, `keyring.bumped`), updated in the same change that moves
  the pin or refreshes the keyring. `pins.toml` is authoritative;
  `RELEASING.md` points at it rather than copying the dates. Age is measured
  from the bump date, never from VCS history. A bump date is a TOML local
  date (`2026-10-09`). When first added, each is the UTC date of the
  default-branch commit that last changed the pin's value, or for the keyring
  the last commit changing a file under `keys/`; never the date of the change
  that adds it.
- **Feeds** — the four external sources and the checks each serves:

  | Feed | Checks it serves |
  |------|------------------|
  | OSV (Open Source Vulnerabilities, `api.osv.dev`) | `playwright-core` advisories; Chromium fixed versions |
  | Node security working group `vuln/core` | Node advisories |
  | CISA (Cybersecurity and Infrastructure Security Agency) KEV (Known Exploited Vulnerabilities catalogue) | Chromium advisories |
  | npm registry | `browsers.json`, from the pinned `playwright-core` tarball, for Chromium |

- **Advisory ID** — the identifier a finding is keyed on: the GHSA (GitHub
  Security Advisory) ID for the `playwright-core` pin, the `vuln/core` entry
  key for Node (an entry
  may name zero, one or several CVEs), and the CVE (Common Vulnerabilities and
  Exposures) ID for Chromium.
- **Checked key** — a primary key or a signing-capable subkey in
  `keys/nodejs-release.asc`, each with its own fingerprint and, optionally,
  expiry. Encryption-only subkeys are not checked keys. A checked key with no
  expiry never produces a key-expiry finding.
- **Finding** — one condition the guard reports, identified by its kind and
  subject:

  | Kind | Identity |
  |------|----------|
  | Advisory | advisory ID + pin |
  | Pin age | pin + pinned version (for Chromium, the revision) |
  | Key expiry | checked key's fingerprint + expiry date |
  | Keyring age | keyring + bump date |

  For pin age, key expiry and keyring age, a new pinned version, a re-signed
  key or a refreshed keyring is a new finding, so a later breach is reported
  even when an earlier issue was closed. An advisory finding carries no
  version: closing its issue is the owner's decision for that pin, so a bump
  that is still affected does not re-alert. A key nearing and then passing
  expiry is one finding.
- **Feed failure** — not a finding: a request to a feed that is unreachable,
  or whose response does not parse or lacks the expected fields. It is
  identified by feed name alone. The expected fields are:

  | Feed | Expected fields |
  |------|-----------------|
  | OSV batch query | `results`, each with `vulns[].id` |
  | OSV record | `id`, `affected`; for a KEV Chromium CVE, a fixed version in `database_specific.unresolved_ranges` (an OSV not-found counts as missing) |
  | `vuln/core` | per entry: `vulnerable`, `patched`, `affectedEnvironments` |
  | KEV | `vulnerabilities`, each with `cveID`, `vendorProject`, `product` |
  | npm registry | the `playwright-core` tarball, and in its `browsers.json` a `browsers` array whose entries carry `revision` and `browserVersion` |

## Requirements

**The job**

- A scheduled daily workflow, also runnable via `workflow_dispatch`, runs the
  guard, which performs every check below and opens issues for findings and
  feed failures.
- Every issue the guard opens carries the `runtime-pin-guard` label and is
  assigned to the owner's GitHub login, read from `RELEASING.md`. The guard
  creates the label if it is missing.
- Each finding opens at most one issue across the issue's whole lifecycle. A
  guard run never duplicates or reopens an issue that already exists for the
  same finding, whether it is open or closed.
- A guard run that opens issues for findings, with every feed responding,
  exits 0; findings are reported through issues, not through a red run.

**Feed failure**

- A feed failure is never treated as "no findings". A failed request belongs
  to the check that issued it, and only that check is skipped, opening no
  advisory issues:

  | Check | Skipped when this fails |
  |-------|-------------------------|
  | `playwright-core` advisories | OSV batch query, or an OSV record fetch for one of its advisories |
  | Node advisories | `vuln/core` |
  | Chromium advisories | KEV, an OSV record fetch for a KEV CVE, or the npm registry |
- Every other check, including the age and expiry checks, still runs and opens
  its issues; the guard run then fails.
- A feed failure ensures one open feed-failure issue exists for that feed.
  Feed-failure issues are deduplicated against open issues only: while one is
  open, later failures add nothing; once it is closed, the next failure opens
  a new one. A persistent outage thus stays visible without a daily stream of
  issues.

**Local inputs**

- The guard evaluates every check before it opens any issue.
- A missing or unparseable local input aborts the guard run without opening
  any issue, with an error naming the input. The local inputs are the bump
  dates, `node.version` and `chromium.revision` in `pins.toml`, the
  `playwright` version in the vendored `package.json`, the owner's GitHub
  login in `RELEASING.md`, and `keys/nodejs-release.asc`. A bump date later
  than the current date counts as unparseable. The red run is the signal; it
  is never treated as "not stale" or "not affected".
- A pin inconsistency aborts the same way: a `browsers.json` whose `browsers`
  array is populated but has no `chromium-headless-shell` entry with
  `chromium.revision`, meaning `pins.toml` and the pinned `playwright`
  version disagree. A missing or empty `browsers` array is an npm-registry
  feed failure instead.

**Advisory matching**

- `playwright-core`: query OSV for the npm packages `playwright-core` **and**
  `playwright` at the pinned version, since Playwright files its advisories
  against `playwright`. One advisory listed under both packages is one
  finding. The issue names the lowest fixed version above the pinned version;
  an advisory with no fix still opens an issue, stating that no fix is
  available.
- Node: match the pinned version against each `vuln/core` entry — affected when
  inside `vulnerable` and outside `patched` — restricted to entries whose
  `affectedEnvironments` include `all`, `darwin` or `linux`. One entry is one
  finding regardless of how many CVEs it names.
- Chromium: match the Chromium pin's browser version against CISA KEV entries
  with `vendorProject == "Google"` and a `product` beginning `Chromium`, taking
  each CVE's fixed version from OSV and comparing the four-part versions
  numerically. Where a record lists several fixed versions, compare against
  the lowest one sharing the browser version's major version, or the lowest
  overall if none does. A pin is affected only when that fixed version is
  strictly higher than the browser version.
- An advisory issue names the advisory ID (and, for Node, the entry's CVEs),
  the affected pin and its version (for Chromium, both the revision and the
  browser version), and the fix: the fixed version for `playwright-core` and
  Chromium, and the `patched` range for Node.

**Age and expiry**

- `RELEASING.md` records `playwright-core`, Node and Chromium as
  security-relevant dependencies with their maximum ages, and names the owner
  on one line of the form `Owner: <name> (@<github-login>)`, which the guard
  parses. `Owner: Jane Doe (@janedoe)` parses; `Owner: Jane Doe` does not.
- The owner is Toby Clemson, for all three pins and the keyring.
- Pins have a maximum age of 90 days, the keyring 365 days, and the key-expiry
  warning window is 60 days.
- The guard opens an issue when a pin's bump date is older than its maximum
  age, when the keyring's bump date is older than its maximum age, or when a
  checked key expires within the warning window or has already expired. A key
  with no expiry opens nothing; it is covered by the keyring's maximum age.
- Days to expiry are the UTC calendar date of the key's expiry timestamp minus
  the current date.
- Every issue names what the owner needs to act: an advisory issue as above;
  a pin-age issue the pin, its version, its age and the owner; a keyring-age
  issue the keyring, its age and the owner; a key-expiry issue the key's
  fingerprint and expiry date; a feed-failure issue the feed and the failed
  request.
- The guard takes the current date as an input, so every age and expiry
  boundary is testable deterministically. It reads and writes issues and
  labels through one interface that tests replace with an in-memory fake
  seeded with open and closed issues.
- A feed-failure issue names every request to that feed that failed in the
  run that opened it.

**Documentation**

- `RELEASING.md` documents each feed, what it covers, its blind spots, and what
  the owner does when each kind of issue opens: advisory, pin age, key expiry,
  keyring age and feed failure. It also tells the owner to update the bump
  date in the same change that moves the pin or refreshes the keyring, and how
  to notice a disabled or stalled schedule.

## Acceptance Criteria

Advisory matching:

- [ ] Given the pinned `playwright-core` version has an advisory filed only
      against npm `playwright`, when the guard runs, then it opens an issue
      naming that advisory, the `playwright-core` pin, its version and the
      fixed version.
- [ ] Given one advisory is listed against both `playwright` and
      `playwright-core` at the pinned version, when the guard runs, then
      exactly one issue opens for it.
- [ ] Given a `playwright-core` advisory affecting the pinned version with
      fixed versions in two ranges, one below and one above the pin, when the
      guard runs, then the issue names the fixed version above the pin.
- [ ] Given a `playwright-core` advisory affecting the pinned version with no
      fix, when the guard runs, then it opens an issue stating that no fix is
      available.
- [ ] Given the pinned Node version is inside a `vuln/core` entry's vulnerable
      range, when the entry's `affectedEnvironments` is each of `all`,
      `darwin` and `linux` in turn, then the guard opens an issue naming the
      entry key, its CVEs, the Node pin, its version and the `patched` range.
- [ ] Given the pinned Node version is inside an entry's vulnerable range but
      the entry affects only `win32`, when the guard runs, then it opens
      nothing for it.
- [ ] Given the pinned Node version is inside both `vulnerable` and `patched`,
      when the guard runs, then it opens nothing for it.
- [ ] Given the pinned Node version is outside an entry's `vulnerable` range,
      or OSV returns a `playwright-core` advisory whose fixed version is at or
      below the pin, when the guard runs, then it opens nothing for it.
- [ ] Given a matching `vuln/core` entry with no CVE, and another naming two
      CVEs, when the guard runs twice, then exactly one issue exists for each
      entry.
- [ ] Given a KEV Chromium CVE whose fixed version is higher than the pinned
      browser version, when the guard runs, then it opens an issue naming the
      CVE, the Chromium revision, the browser version and the fixed version.
- [ ] Given a KEV Chromium CVE whose fixed version equals the pinned browser
      version, when the guard runs, then it opens nothing for it.
- [ ] Given a pinned browser version of `120.0.6099.9` and a KEV Chromium CVE
      fixed in `120.0.6099.10`, when the guard runs, then it opens an issue
      (numeric, not lexical, comparison).
- [ ] Given a pinned browser version of `121.0.6167.5` and a KEV Chromium CVE
      whose OSV record lists fixed versions `120.0.6099.300` and
      `121.0.6167.85`, when the guard runs, then it opens an issue naming
      `121.0.6167.85`.
- [ ] Given the same OSV record and a pinned browser version of
      `122.0.6261.5`, when the guard runs, then it opens nothing (the lowest
      overall, `120.0.6099.300`, is not higher); given a pinned browser
      version of `119.0.6045.5`, then it opens an issue naming
      `120.0.6099.300`.
- [ ] Given a Chromium CVE fixed after the pinned browser version that is not
      in KEV, when the guard runs, then it opens nothing for it.
- [ ] Given a KEV entry with `vendorProject == "Google"` whose `product` does
      not begin `Chromium`, when the guard runs, then it opens nothing for it.

Age and expiry:

- [ ] For each of `playwright-core`, Node and Chromium: given its bump date is
      91 days before the current date, when the guard runs, then it opens an
      issue naming the pin, its version (the declared `playwright` version,
      `node.version`, or `chromium.revision`), its age and the owner; at
      exactly 90 days it opens nothing.
- [ ] Given the keyring's bump date is 366 days before the current date, when
      the guard runs, then it opens an issue naming the keyring, its age and
      the owner; at exactly 365 days it opens nothing.
- [ ] Given a checked key 60 days from expiry, when the guard runs, then it
      opens an issue naming the key's fingerprint and expiry date; at 61 days
      it opens nothing.
- [ ] Given a checked key whose expiry timestamp is 23:59 UTC on the date 61
      days after the current date, when the guard runs, then it opens
      nothing.
- [ ] Given a checked key that expires on or expired before the current date,
      when the guard runs, then it opens a key-expiry issue.
- [ ] Given a primary key expiring in two years with a signing subkey
      expiring in 30 days, when the guard runs, then it opens one issue naming
      the subkey's fingerprint and expiry date.
- [ ] Given a primary key expiring in two years with an encryption-only
      subkey expiring in 30 days, when the guard runs, then it opens nothing.
- [ ] Given a key with no expiry, when the guard runs, then it opens nothing
      for it.

Local inputs:

- [ ] Given a pin or the keyring with a missing or unparseable bump date in
      `pins.toml`, when the guard runs, then it exits non-zero with an error
      naming it and opens no issue.
- [ ] Given a missing or unparseable `node.version`, `chromium.revision`,
      declared `playwright` version, `Owner:` line in `RELEASING.md` (for
      example `Owner: Jane Doe`), or `keys/nodejs-release.asc`, when the guard
      runs, then it exits non-zero with an error naming the input and opens no
      issue.
- [ ] Given a `browsers.json` with a populated `browsers` array but no
      `chromium-headless-shell` entry for `chromium.revision`, when the guard
      runs, then it exits non-zero with an error naming the revision and opens
      no issue.

Issue lifecycle:

- [ ] Every issue the guard opens, of every kind, carries the
      `runtime-pin-guard` label and is assigned to the owner's GitHub login
      recorded in `RELEASING.md`.
- [ ] Given the `runtime-pin-guard` label does not exist, when the guard runs
      with a finding, then it creates the label and the opened issue carries
      it.
- [ ] Given an issue already exists for the same finding, open or closed, when
      the guard runs again, then it opens no new issue.
- [ ] Given an issue exists for advisory A on the Node pin, when advisory B
      also matches the Node pin, then exactly one new issue opens, for B.
- [ ] Given a closed issue for advisory A on the Node pin, when Node is bumped
      to a version still affected by A, then the guard opens nothing for A.
- [ ] Given a closed pin-age issue for an earlier version of a pin, when the
      current version exceeds its maximum age, then a new issue opens.
- [ ] Given a closed key-expiry issue for a key expiring on D1, when the key is
      re-signed to expire within the warning window on D2, then a new issue
      opens.
- [ ] Given a closed key-expiry issue opened during the warning window, when
      the key then passes its expiry, then the guard opens nothing more.
- [ ] Given a closed keyring-age issue, when the keyring is refreshed and its
      new bump date also exceeds the maximum age, then a new issue opens.
- [ ] Given every pin and key is clear, when the guard runs, then it opens
      nothing and exits 0.
- [ ] Given findings exist and every feed responded, when the guard runs, then
      it opens their issues and exits 0.

Feed failure, with one fixture per feed for each of unreachable, unparseable
and missing-field responses:

- [ ] Given a request to feed F fails, when the guard runs, then it exits
      non-zero, opens no advisory issues for the check whose request failed,
      and still opens issues for every other check's findings.
- [ ] Given the npm registry fails, or returns a `browsers.json` with no
      `browsers` entries, while KEV and OSV respond, when the guard runs, then
      no Chromium advisory issue opens and an npm-registry feed-failure issue
      opens.
- [ ] Given a KEV Chromium CVE for which OSV returns not-found, or a record
      without a fixed version, and a `playwright-core` advisory present in
      OSV, when the guard runs, then it exits non-zero, opens one OSV
      feed-failure issue, opens no Chromium advisory issue, and opens the
      `playwright-core` advisory issue.
- [ ] Given OSV's batch query succeeds but the record fetch for one of its
      `playwright-core` advisories fails, when the guard runs, then it opens
      no `playwright-core` advisory issue, opens an OSV feed-failure issue, and
      still runs the Chromium check.
- [ ] Given `vuln/core` fails, when the guard runs, then it opens no Node
      advisory issue and still runs the `playwright-core` and Chromium checks.
- [ ] Given feed F fails and no feed-failure issue for F exists, when the
      guard runs, then it opens one naming F and the failed request (URL or
      record ID).
- [ ] Given two OSV record fetches fail in one run, one for a
      `playwright-core` advisory and one for a KEV Chromium CVE, when the
      guard runs, then exactly one OSV feed-failure issue opens, naming both
      record IDs.
- [ ] Given a `browsers.json` with an empty `browsers` array, when the guard
      runs, then it opens an npm-registry feed-failure issue and does not
      abort.
- [ ] Given an open feed-failure issue for F, when F fails again, then the
      guard opens nothing more for F.
- [ ] Given only a closed feed-failure issue for F, when F fails again, then
      the guard opens a new feed-failure issue for F.

Workflow and documentation:

- [ ] The workflow declares `schedule` (daily) and `workflow_dispatch`
      triggers, runs the guard, grants `issues: write`, and sets a
      `concurrency` group.
- [ ] Each time a change to the guard merges, a `workflow_dispatch` run on the
      default branch concludes successfully, or fails only through feed
      failures for which an open feed-failure issue exists after the run;
      afterwards the `runtime-pin-guard` label exists and every issue the run
      opened carries it and is assigned to the owner.
- [ ] Within 48 hours of the workflow first merging, a run triggered by
      `schedule` appears in the workflow's history and meets the same
      condition.
- [ ] Once advisory matching is merged, a dispatched run reaches all four
      feeds and parses their responses without opening a feed-failure issue.
      If one opens, the owner requests the same URL from outside GitHub
      Actions within an hour; the criterion passes only if that also fails,
      otherwise the run is re-dispatched.
- [ ] Given a bump date later than the current date, or one not in TOML local
      date form (`2026-13-01`), when the guard runs, then it exits non-zero
      naming it and opens no issue.
- [ ] `pins.toml` carries a bump date for each pin and the keyring, each equal
      to the UTC date of the default-branch commit that last changed that
      pin's value, or for the keyring the last commit changing `keys/`.
- [ ] `RELEASING.md` carries a parseable `Owner:` line, each pin's and the
      keyring's maximum age, the warning window, where the bump dates live and
      the instruction to update them in the same change, the four feeds and
      what each covers, the KEV-only Chromium filter, the blind spots listed
      under Assumptions, the owner's action for each issue kind, and how to
      notice a disabled schedule or one with no run in the last 48 hours.

## Open Questions

Decided by the owner before planning:

- What is the owner's GitHub login?

Decided during implementation:

- Which OpenPGP parser reads key and subkey expiry: the `gpg` runner the
  release workflow already uses to verify Node (`tasks/shared/vendor/nodejs`)
  or a Python library? Whichever is chosen must be pinned or probed on the
  runner as that `gpg` is.
- What is the guard's `mise` task called?

## Dependencies

- Builds on: 0196 (`done`) — shipped `pins.toml`, the `playwright-core` pin,
  the `playwright-core` fetch from the npm registry and the keyring this guard
  reads.
- Hand-over from 0196: the 0196 plan's scheduled "stale-pin guard" criterion
  (this item's age-and-expiry checks) and its `RELEASING.md` owner and
  maximum-age records were not built under 0196 and are delivered here.
- Relates to: 0219 — builds an open-or-comment step that opens a GitHub issue
  per dedup key or comments on the open one, through a dedicated measurement
  GitHub App. Whichever of the two lands second reuses the other's step;
  neither waits for the other.
- Owner login: planning needs the owner's GitHub login (see Open Questions).
- External feeds: OSV, the Node security working group feed (served from
  `raw.githubusercontent.com`), CISA KEV and the npm registry. None has a
  service-level agreement, and the first three have no versioned schema; a
  failure skips the checks it serves and fails the guard run. Chromium fixed
  versions come from OSV's `database_specific.unresolved_ranges`, a
  source-specific extension outside OSV's core schema; a change there shows
  as a persistent OSV feed failure.
- GitHub Actions scheduling: cron runs can be delayed or skipped under load,
  and GitHub disables scheduled workflows in a public repository after 60 days
  without repository activity. A disabled schedule stops every check
  silently, hence the `RELEASING.md` instruction.
- Repository: repository or organisation workflow-token settings must allow
  `issues: write`, and the owner's login must be an assignable collaborator.
  Scheduled workflows run only from the default branch, so token, label and
  assignee problems surface only after merge.
- `RELEASING.md` becomes a machine-read input: the guard parses the `Owner:`
  line defined under Age and expiry, so later edits must keep its format.
- `pins.toml` readers: `tasks/shared/vendor/pins.py`,
  `tasks/shared/vendor/trust_anchors.py` and `cli/launcher/build.rs` look
  fields up by key, so the new `bumped` keys and `[playwright-core]` and
  `[keyring]` tables need no change to them;
  `tests/unit/tasks/test_tree_artifact_pins.py` must still pass.
- Trust-anchor approval: none. The 0196 plan's trust-anchor section designed
  a required CI job that fails any change to `pins.toml` or `keys/` without a
  second approver from a named reviewer team, but it was never built:
  `.github/workflows/main.yml` runs only `vendor:check-trust-anchors`, which
  rejects placeholder values, and there is no `CODEOWNERS`. Bump-date changes
  therefore merge under ordinary review. Building that gate is separate work
  and does not block this item.
- Tooling: an OpenPGP parser to read key and subkey expiry (see Open
  Questions), and the `gh` CLI on the runner, authenticated with the workflow
  token.

## Assumptions

- Chromium exposure that matters is in-the-wild exploitation. Unexploited
  Critical CVEs are not caught; covering them needs the National Vulnerability
  Database (NVD — rate-limited, lagging Common Platform Enumeration (CPE)
  enrichment, noisy version ranges) or scraping the Chrome Releases blog.
- The vendored browser can load content the project does not control, so
  browser CVEs are real exposure rather than test-only.
- Advisory issues stay open until a human closes them; auto-closing once a pin
  moves past the fix is out of scope.
- Keeping a bump date in step with its pin is a manual step in the same
  change, not enforced by a check. Forgetting it errs safe: the pin looks
  older than it is and the age issue opens early.
- OSV can lag a fresh KEV Chromium entry. Until it records a fixed version,
  the Chromium check is skipped and one OSV feed-failure issue stays open;
  this is accepted over guessing whether the pin is affected.

## Technical Notes

- Pins live in `pins.toml` (`chromium.revision`, `node.version`); the
  `playwright` version is declared in the vendored `package.json`, and
  `tasks/shared/vendor/upstream.py` fetches the matching `playwright-core`
  tarball, which carries `browsers.json`, from the npm registry. Publisher keys
  are `keys/nodejs-release.asc` and `keys/npm-registry.pem`.
- OSV: `POST https://api.osv.dev/v1/querybatch` returns only IDs and
  `modified`; fetch each full record with `GET /v1/vulns/{id}`. No auth or
  documented rate limit. Chromium CVE records carry their fixed versions in
  `database_specific.unresolved_ranges`, which `/v1/query` cannot match on, so
  they are read by CVE ID.
- Node feed: `https://raw.githubusercontent.com/nodejs/security-wg/main/vuln/core/index.json`.
  `is-my-node-vulnerable` wraps it but does not report which CVEs matched, so
  parse it directly.
- KEV: `https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json`;
  entries carry no version fields, hence the OSV join.
- Dedup: the `runtime-pin-guard` label plus a hidden body marker —
  `<!-- finding: <kind> <identity> -->` for findings and
  `<!-- feed-failure: <feed> -->` for feed failures — checked against
  `gh issue list --label runtime-pin-guard --state all --json
  number,state,body` filtered locally rather than via search, which lags. The
  workflow needs `permissions: issues: write` and a `concurrency` group so
  overlapping runs cannot double-create.
- Keep the matching logic in the `tasks/` Python toolchain so it is driven
  test-first against recorded feed fixtures, with the workflow a thin
  scheduler.

## Drafting Notes

- The 0196 age-and-expiry guard was folded into this item rather than raised
  separately, because it shares the scheduled job, the issue plumbing and the
  `RELEASING.md` section. The title and slug were widened to match. Splitting
  it was considered at review and declined. Delivery may land in two changes,
  the second merging only after the first. The first meets every Age and
  expiry criterion; every Local inputs criterion except the `browsers.json`
  one; every Issue lifecycle criterion except the two advisory ones; the
  workflow, `pins.toml`, dispatched-run and scheduled-run criteria; and the
  `RELEASING.md` criterion's owner, maximum-age, bump-date, schedule and
  non-advisory owner-action clauses. The second meets the rest: advisory
  matching, feed failure, the `browsers.json` and advisory lifecycle
  criteria, a repeat of the dispatched-run criterion, the live-feed
  criterion, and the `RELEASING.md` feed, KEV-filter, blind-spot and advisory
  owner-action clauses. If the second change grows large, Chromium advisory
  matching can land last on its own; failures are already isolated per check.
- The trust-anchor second-approver gate the 0196 plan designed is unbuilt.
  Keeping bump dates in `pins.toml` was chosen while that gate was assumed to
  exist; the choice stands without it, and the gate merits its own work item.
- The original Context claimed the stale-pin half was already handled; it was
  not built, and the Context was corrected.
- Chromium matching was restricted to KEV because Playwright pins a pre-stable
  build that is never patched, so "older than a fixed version" holds for dozens
  of CVEs within weeks of every Playwright release; a severity threshold does
  not help, since Chromium severities are dominated by High.
- `ffmpeg` and `winldd` from `browsers.json` were excluded: `ffmpeg` only
  encodes Playwright's own screencast frames, and `winldd` is a Windows-only
  dependency checker outside the supported platforms. `chromium-headless-shell`
  shares Chromium's `browserVersion` and is covered implicitly.
- GitHub Advisory Database was not chosen: it duplicates OSV's npm coverage and
  covers neither Node core nor Chromium.
- `keys/npm-registry.pem` carries no expiry in PEM form, so key expiry checks
  only the Node release keyring; the npm key is covered by the keyring's
  maximum age.
- Age is measured from an explicit bump date rather than VCS history so that
  reformatting `pins.toml` cannot reset it and fixtures stay deterministic.
  The dates sit in `pins.toml` beside the pins they describe.
- 90 days for pins reflects Playwright's roughly monthly releases.
- `kind` stays `task` despite the size; the two-change delivery above records
  it. The title names the runtime pins only; the keyring guards are in scope,
  as Summary and Terms state.

## References

- Designed in: `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`
  (trust-anchor guards; Removal sweep, follow-up work items)
- Review: `meta/reviews/work/0225-scheduled-advisory-and-age-guards-for-the-vendored-runtime-pins-review-1.md`
- Related: 0196, 0162 (`cargo-deny` advisories for Rust crates)
- OSV API: https://google.github.io/osv.dev/post-v1-querybatch/
- Node security feed: https://github.com/nodejs/security-wg/tree/main/vuln/core
- CISA KEV: https://www.cisa.gov/known-exploited-vulnerabilities-catalog
- Playwright `browsers.json`: https://github.com/microsoft/playwright/blob/main/packages/playwright-core/browsers.json
- Playwright advisory filed against `playwright`: https://osv.dev/vulnerability/GHSA-7mvr-c777-76hp
