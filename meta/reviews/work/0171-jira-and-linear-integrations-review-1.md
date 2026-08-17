---
type: work-item-review
id: "0171-jira-and-linear-integrations-review-1"
title: "Work Item Review: Jira and Linear Integrations"
date: "2026-08-17T10:13:53+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0171"
work_item_id: "0171"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [rust, jira, linear, integrations, cutover]
last_updated: "2026-08-17T12:20:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Jira and Linear Integrations

**Verdict:** REVISE

0171 is exceptionally well specified on its cutover half — named files,
exact counts, and criteria a verifier can settle with a single shell
command — and the ownership split with 0174 is negotiated in both
directions. The client-building half is specified to a materially lower
standard: the integration-skill retirement is asserted only in
Assumptions, its primary acceptance criterion names an oracle this same
story deletes, and the two most failure-prone requirements (four
per-operation exit-code tables, the timeout and page-cap bounds) have no
criterion at all. Underneath both sits a sizing problem — three
independently deliverable efforts share one `done` gate, in an epic that
has already split two siblings for exactly this shape.

### Cross-Cutting Themes

- **The integration half is a second-class citizen** (flagged by:
  clarity, completeness, scope, testability) — deletion of the 22 Jira
  and 12 Linear scripts is never a requirement, no criterion asserts
  their directories are gone, the `SKILL.md` bodies are never repointed
  (only `allowed-tools`), and AC1 verifies against "the repointed
  integration suites" — an artefact the same document withdraws.
- **Requirements without criteria** (flagged by: completeness,
  testability) — the four non-nested exit-code tables and the
  timeout/page-cap bounding have no acceptance criterion, so the two
  requirements most likely to be silently under-implemented have nothing
  defining when they are done.
- **Criteria that cannot fail** (flagged by: testability, scope) — AC5
  accepts any fate for the four port-less capabilities as long as it is
  recorded, including dropping the identifier-safety check the
  Requirements mandate unconditionally.
- **Real-tracker verification is unexecutable as written** (flagged by:
  dependency, scope, testability) — provisioning a credentialed Jira
  project and Linear team is an undischarged prerequisite behind the sole
  Open Question, yet Dependencies asserts every blocker is discharged.
- **Size** (flagged by: scope, and visible in the item's own Drafting
  Notes) — each enrichment pass discovers material the previous scope
  missed; 0136 records 0169 and 0173 being split on that same signature.

### Findings

#### Critical

- 🔴 **Clarity / Completeness / Testability / Scope**: The integration-skill
  half is unspecified, and AC1's oracle is deleted by this story
  **Location**: Summary, Requirements, Acceptance Criteria (first)
  The Summary limits the cutover to `work-item-*.sh`; the retirement of
  `skills/integrations/{jira,linear}/scripts/`, their libraries, their
  bash suites and their Python mock servers appears only obliquely (inside
  the `_EXPECTED_INTEGRATIONS_SUITES` requirement) and once in
  Assumptions — never as a requirement, never as a criterion. AC1 then
  verifies eight flows per provider plus ADF↔markdown, JQL and GraphQL
  "against the repointed integration suites", which cannot be the bash
  suites if those retire and which no requirement establishes in the Rust
  lane. The largest deliverable in the story therefore has no defined
  pass condition, and an implementer following the checklist literally
  can ship the Rust clients while leaving two live implementations per
  provider.

- 🔴 **Testability / Completeness**: Four exit-code tables and the
  timeout/page-cap bounds have no acceptance criterion
  **Location**: Requirements (exit-code tables; port call bounding)
  The Requirements warn that the create and update provable sets are "not
  nested in either direction" (Linear code 34 retryable on `create`,
  terminal on `update`; codes 18, 23, 25, 27, 29 the other way) and that
  the port must reproduce `--max-time 30`/`60` and `_WIFR_PAGE_CAP=20`
  from inside because a caller cannot add them. Neither has a criterion.
  A misclassified code silently retries an unretryable mutation or
  terminates a retryable one, and an unbounded read path hangs
  `/list-work-items` — both shippable with every criterion green.
  Assumptions compound this by leaving table coverage conditional on what
  `wiremock` can express.

- 🔴 **Scope**: Two provider integrations and a large cutover bundled into
  one story
  **Location**: Summary
  The Summary joins the efforts with a sequencing word — build the two
  client crates and binaries, "Then perform the work-item cutover". The
  two clients are independent of each other (different protocols, crates,
  binaries, skill directories, mock scenarios, and separate non-nested
  mapping tables) and neither is user-visible until the cutover flips the
  callers. Three separately deliverable and revertible efforts share one
  review, one branch and one `done` gate. Splitting does not recreate the
  dead-window problem the 2026-08-10 note rejected, because the bash path
  stays live until the cutover child lands.

#### Major

- 🟡 **Clarity / Testability**: The identifier-safety check is mandated,
  then made droppable
  **Location**: Requirements, Acceptance Criteria (fifth)
  A requirement carries the check over unconditionally ("An unsafe
  identifier is a `Terminal` failure, not an `Ok`"), yet AC5 lists it
  among four capabilities that may be "re-sited, dropped or carried
  forward as a recorded decision". The same requirement's list enumerates
  only three capabilities plus a closing note. An implementer can satisfy
  AC5 by recording a decision to drop the check, reintroducing control
  characters and leading `---`/`#` into unquoted YAML frontmatter.

- 🟡 **Testability / Scope**: The capability-gap criterion cannot fail, and
  leaves delivered scope open-ended
  **Location**: Acceptance Criteria (fifth), Requirements
  AC5 is satisfied by dropping all four capabilities and writing that
  down. It names no location for the record and no behavioural check per
  outcome, despite two being load-bearing today: unkeyed `search` is how
  `/sync-work-items` lists remote issues with no local item, and the
  update `--dry-run` is what `--preview` validates pushes with (0194's
  `--preview` explicitly does not discharge it). The third permitted
  outcome — "carry it as an additive port item" — would reopen the port
  0204 froze at six items and pull four crates into the change, so the
  item's size is unknowable at planning time.

- 🟡 **Dependency / Testability**: The credentialed-target gate is
  unprovisioned, unexecutable, and narrower than the risk
  **Location**: Dependencies, Acceptance Criteria (contract harness),
  Open Questions
  Dependencies opens "All blockers discharged as of 2026-08-17 … the item
  is startable whole", but a scratch Jira project, a Linear team, API
  tokens and (if CI) repository secrets do not exist, and the sole Open
  Question decides where they live — and therefore whether a CI workflow
  change is in scope. The criterion also names only `create` → `show` and
  `update` → `show`, while Dependencies warns that a quietly dropped
  operation is undetectable below a harness exercising all four against
  the real client.

- 🟡 **Scope**: Declared `kind: story` is inappropriate for epic-scale
  scope
  **Location**: Frontmatter: kind
  22 requirement bullets and 17 criteria spanning two library crates, two
  dispatched binaries, three retirement clusters, a fixture relocation,
  eleven test conversions, four artefact repoints, build-system surgery, a
  new conversational flow and external account provisioning. 0136's other
  children are one CLI or one fix each; 0169 and 0173 were split on the
  signature this item now shows — each pass discovering material the last
  scope missed.

- 🟡 **Scope**: The cutover half itself bundles three independent
  retirement clusters
  **Location**: Requirements
  The work scripts (with fixtures, eleven tests, three SKILL repoints),
  the integration scripts and suites, and the build-system floors and
  `SHELL_LIBRARIES` entries retire on different prerequisites — cluster
  (a) on both clients, (b) on the respective provider's client only, (c)
  mechanically on whichever landed. Bundling forces each to wait on the
  others across three test lanes and the CI shell gate at once.

- 🟡 **Scope**: The conversational conflict flow is a separate user-facing
  increment
  **Location**: Requirements
  Parsing the conflict report, rendering context, collecting choices and
  re-invoking with `--resolve` shares no crate and no test lane with the
  clients or the deletions, and its dependency (0194's report format and
  exit codes) is already discharged. It cannot be reviewed or reverted on
  its own.

- 🟡 **Dependency**: Jira REST and Linear GraphQL are absent from
  Dependencies
  **Location**: Dependencies
  The story is two HTTP clients against third-party APIs, yet neither
  external system is recorded with its availability, rate-limit,
  complexity-cap or API-versioning implications — while upstream 0204,
  which only describes them, does exactly that. Two criteria can fail for
  reasons entirely outside the change.

- 🟡 **Dependency**: The 0174 blocking edge exists in one direction only
  **Location**: Dependencies, Frontmatter: relates_to
  0174 lists `work-item:0171` in `blocked_by`; 0171 records 0174 only
  under "Relates to" and carries no `blocks` field. Anyone scheduling
  from 0171 cannot see the downstream cleanup waiting on it, and a tracker
  sync trusting this frontmatter drops the dependant.

- 🟡 **Dependency**: 0194 is asserted complete but its record reads `ready`
  **Location**: Dependencies, Drafting Notes
  The prerequisite the whole cutover half rests on is, on the record, not
  done — and the Drafting Notes concede it while Dependencies calls it
  complete. Any readiness check reading statuses rather than prose will
  disagree with this item about whether it can start.

- 🟡 **Testability**: The baseline-corpus criterion has no seeding
  procedure, and omits the dangerous case
  **Location**: Acceptance Criteria (baseline corpus)
  "Against matching remote records" gives no way to make a bash-generated
  corpus match live tracker state, so the precondition is unreachable.
  Nor does it require the absent-description case — literal `null` for
  Jira, empty line for Linear — that Requirements single out as the one a
  deserialiser gets wrong, with mass reclassification as the consequence.

- 🟡 **Testability**: The conflict-rendering criterion rests on a
  subjective threshold with no verification lane
  **Location**: Acceptance Criteria (conflict flow), Requirements
  "Enough local and remote context for the user to judge" enumerates no
  fields, and nothing states whether a conversational SKILL flow is
  checked by a test, a manual walkthrough, or inspection. The half that
  makes conflict resolution possible at all can be argued complete with a
  one-line render.

- 🟡 **Clarity**: Bare numeric codes have no named namespace
  **Location**: Requirements (exit-code tables)
  "Linear code 34", "codes 18, 23, 25, 27 and 29", then "a single
  status-to-class table is wrong" — the namespace is never named, and 34
  is not a valid HTTP status, so "status" misdirects. An implementer could
  key the Rust mapping off HTTP status instead of transport exit code,
  producing exactly the misclassification the requirement exists to
  prevent.

#### Minor

- 🔵 **Clarity**: "The rule they encode" is never stated, and "provable
  set" / "provably pre-transmission" are undefined
  **Location**: Requirements
  The instruction "the tables win" is followable, but a reviewer cannot
  judge whether a divergence found during porting is the intended
  conservatism or a table bug.

- 🔵 **Clarity**: "The tagged, network-touching filter" is referenced
  before anything names it
  **Location**: Requirements
  No tag, nextest filter expression, cargo feature or `#[ignore]`
  convention is named, leaving AC7's "the default `cargo test` makes no
  network call" without a mechanism.

- 🔵 **Clarity**: Cluster of unresolved referents
  **Location**: Requirements
  "Behind the seam", "the value returned" (by which operation?), "rely on
  them" (the entries or `jq`/`curl`?), and "holding the two
  implementations in step" (description or instruction?) each admit more
  than one reading.

- 🔵 **Clarity**: Two dirty-guard callers mapped onto three Rust paths
  without pairing
  **Location**: Requirements
  The one behaviour whose loss means overwriting a user's uncommitted work
  item is the part left implicit.

- 🔵 **Completeness**: Open Questions is sparse relative to the decisions
  the Requirements defer
  **Location**: Open Questions
  The four capability fates and the `EXIT_CODES.md` siting are deferred
  inside prose requirements rather than surfaced where a planner looks;
  the one recorded question has no owner and no deadline.

- 🔵 **Completeness**: Status remains `draft` despite the item declaring
  itself startable whole
  **Location**: Frontmatter: status

- 🔵 **Dependency**: Ordering between the client half and the cutover half
  is not stated
  **Location**: Requirements
  An implementer could relocate the corpus or delete the bash surface
  before projection fidelity is proven, losing the only oracle that would
  have caught the regression.

- 🔵 **Dependency**: New crate ingest is not captured as a coupling to the
  Rust policy lanes
  **Location**: Technical Notes
  `wiremock-rs`, rustls and their trees must clear `deny.toml`'s licence
  and advisory policy; no requirement expects a policy change.

- 🔵 **Dependency**: Two new sub-binaries couple to the release and
  attribution pipeline
  **Location**: Dependencies
  0165's upload set and signed `manifest.json`, and 0203's attribution
  artefact, are unreferenced. A binary that registers locally but is
  absent from the manifest is undiscoverable until a launcher fetches it.

- 🔵 **Dependency**: The 0170 entry understates the repoint's dependency on
  its command surface
  **Location**: Dependencies
  "No direct dependency" is inaccurate in the direction that matters: the
  repoint consumes `accelerator work create`/`list`, which 0170 delivers.

- 🔵 **Scope**: Credentialed tracker provisioning spans a different
  ownership domain
  **Location**: Open Questions

- 🔵 **Scope**: Integrations-side scope stated by reference while
  work-side is enumerated
  **Location**: Requirements

- 🔵 **Testability**: "No duplication of API logic" has no pass/fail
  procedure
  **Location**: Acceptance Criteria (second)
  The structural guarantee the story exists to deliver is a judgement a
  generous reader can always call satisfied.

- 🔵 **Testability**: "Assertions unchanged in substance" is arguable
  **Location**: Acceptance Criteria (eleven shellout tests)
  A weakened assertion is the cheapest way to make a conversion pass, and
  these tests are the cutover's only regression guard.

- 🔵 **Testability**: Three cleanup requirements have no criterion beyond
  the catch-all
  **Location**: Requirements vs Acceptance Criteria
  `_EXPECTED_INTEGRATIONS_SUITES`, the eight `SHELL_LIBRARIES` entries and
  the cross-skill `jq`/`curl` audit — the first and third with nothing
  covering them at all.

#### Suggestions

- 🔵 **Clarity**: Unnamed actor for `E_DISPATCH_*` taxonomy ownership, and
  "a dispatch token" singular for two new commands
  **Location**: Requirements, Dependencies

- 🔵 **Completeness**: Story context describes the code surface, not whose
  need is met
  **Location**: Context
  The user-facing motivation appears only incidentally, mid-requirement
  ("until this lands no caller can resolve a conflict").

### Strengths

- ✅ Requirements and criteria name concrete, checkable artefacts —
  thirteen production scripts, five suites, eleven Rust tests, eight
  `SHELL_LIBRARIES` entries, `_EXPECTED_WORK_SUITES = 5` by symbol and
  line — leaving little room for interpretation on the cutover half.
- ✅ Cutover criteria are mechanically decidable rather than judgemental:
  `ls skills/work/scripts/*.sh` matching nothing, the fixtures directory
  not existing, a grep of `cli/` returning no hits.
- ✅ The highest-risk fidelity case is pre-empted explicitly: the absent
  description projecting as literal `null` for Jira and an empty line for
  Linear, with the mass-reclassification consequence stated.
- ✅ Where two sources of truth could conflict, the item says which wins
  ("the tables win"), removing an interpretation choice from the
  implementer.
- ✅ Several criteria use precondition/action/outcome framing well,
  including the awkward detail that `unresolved` lines must be read on
  exit `71` as well as `4`.
- ✅ The boundary with 0174 is negotiated in both directions and dated, so
  no suite floor or library entry can be cleared twice or missed.
- ✅ The four obligations inherited from 0194's deliberately unfinished
  cutover are enumerated rather than left implicit, as is the guard gap
  0204 handed over (a default-bodied trait method escaping `cargo
  public-api`).
- ✅ Drafting Notes carry a real decision log with dates and a rejected
  alternative (an interim shim over the bridge scripts) and its reasoning.
- ✅ Doc-comment updates are pinned to named sites, turning a normally
  unverifiable tidy-up into a checklist.
- ✅ Every template section is present and substantively populated, with
  frontmatter valid for a story and empty linkage slots correctly omitted.

### Recommended Changes

1. **Decompose into siblings under 0136** (addresses: the bundling
   critical, `kind: story`, the three-cluster and conflict-flow majors)
   Three or four children: `jira-client` + `accelerator-jira`,
   `linear-client` + `accelerator-linear`, the cutover (deletions, fixture
   relocation, test conversions, skill repoints, floors), and optionally
   the conversational conflict flow — which depends only on 0194 and could
   land first. Attach each cluster's floor and `SHELL_LIBRARIES` edits to
   the child that orphans them, preserving 0174's lockstep rule within
   each child.

2. **Specify the integration half to the same standard as the work half**
   (addresses: the AC1-oracle critical, the integrations-by-reference
   minor) Add a requirement naming the script directories, libraries and
   bash suites to delete and the two `SKILL.md` bodies to repoint; add a
   criterion asserting the directories are gone. Replace AC1's oracle with
   one that outlives the bash suites — per flow, a `wiremock`-backed test
   asserting the outgoing request (method, path or GraphQL document, body)
   and the parsed response against a fixture captured from today's bash
   flow — and name ADF↔markdown conversion explicitly.

3. **Add criteria for the two uncovered requirements** (addresses: the
   exit-code/timeout critical) A table-driven criterion enumerating every
   code in all four bash tables and asserting the `TrackerError` class
   *per operation*, with Linear 34 and 18/23/25/27/29 named as required
   cases; and an observable-threshold criterion for the bounds — a
   never-responding `wiremock` endpoint failing within ~35s (Jira) and
   ~65s (Linear), and a 21-page fixture stopping at 20.

4. **Remove the identifier-safety check from AC5 and give it its own
   criterion** (addresses: the mandated-then-droppable major, the
   enumerates-three clarity finding) Enumerate the four rejection cases —
   control character, newline, leading `---`, leading `#` — each producing
   a `Terminal` failure with no frontmatter written. Fix the
   "four capabilities" list to enumerate four, and state that this one's
   fate is already decided.

5. **Close the four capability fates before pickup, and make AC5 fail if
   behaviour is lost** (addresses: the self-certifying-criterion major)
   Record each decision in a named location, and add outcome-conditional
   behavioural criteria — `/sync-work-items` still lists remote issues
   with no local item; `--preview` still surfaces an unresolvable Jira
   project or an invalid payload before any mutation.

6. **Resolve the secrets question and record the tracker target as a
   prerequisite** (addresses: the credentialed-gate major) Name the runner
   (a CI job with named secrets, or a documented manual step and where its
   evidence lands), qualify the "all blockers discharged" sentence, add
   the provisioning as an explicit prerequisite, and extend the criterion
   to all four operations including `fetch_all` partition totality and the
   read-never-`Terminal` rule.

7. **Repair the dependency graph** (addresses: the 0174, 0194, 0204 and
   0170 findings) Add `blocks: ["work-item:0174"]` and 0204 to the
   frontmatter, flip 0194's status to `done` (or name its residue as a
   live blocker here), record Jira REST and Linear GraphQL as external
   systems with their rate and complexity limits, note the `deny.toml` and
   release-manifest couplings, and reword the 0170 entry to record the
   real discharged coupling.

8. **Tighten the arguable criteria** (addresses: the "no duplication",
   "unchanged in substance", conflict-rendering and cleanup findings)
   Restate duplication structurally (no `reqwest` usage or provider
   endpoint outside the client crates), make assertion parity countable
   against the committed goldens, enumerate the minimum rendered conflict
   fields, and extend the floor criterion to name
   `_EXPECTED_INTEGRATIONS_SUITES` and the eight `SHELL_LIBRARIES`
   entries.

9. **Fix the local clarity defects** (addresses: the clarity minors and
   suggestion) Name the exit-code namespace on first use, state the rule
   the tables conservatively encode, name the network-lane filter, pair
   each dirty-guard caller with its Rust path, resolve "behind the seam" /
   "the value returned" / "rely on them", name the crate that owns
   `E_DISPATCH_*`, and pluralise the dispatch tokens.

10. **Advance `status` and enrich Context** (addresses: the two
    completeness minors) Move `status` to `ready` for whichever children
    are startable, drop the stale parenthetical about 0194, and add a
    sentence naming the beneficiary and today's pain.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually dense but mostly precise: it names
concrete file paths, function names and counts, and states outcomes as
observable system states rather than vague properties. Clarity breaks down
in three places — a contradiction over the fate of the identifier-safety
check (mandated as a requirement, yet listed among the four capabilities
that may be "dropped"), an unstated scope boundary around the jira/linear
bash scripts and suites (the Summary limits deletion to `work-item-*.sh`,
while other sections and AC1 assume the integration suites both retire and
remain as verification), and a set of bare numeric codes whose namespace is
never named. Several noun phrases ("the tagged, network-touching filter",
"behind the seam", "the rule they encode") are used before or without
definition.

**Strengths**:

- Requirements and Acceptance Criteria overwhelmingly name concrete,
  checkable artefacts, leaving little room for interpretation about what is
  being changed.
- Outcomes are stated as observable system states rather than desired
  properties.
- Potential misreadings are pre-empted explicitly: the absent-description
  projection, and `RemoteTimestamp`'s "never `Reported("")`" rule.
- Drafting Notes give a dated, reasoned audit trail of scope changes.
- Where two sources of truth could conflict, the item states which wins.

**Findings**:

- **major / high** — *The "four bridge capabilities" list enumerates three,
  and its fourth member contradicts a mandatory requirement*
  (Requirements / Acceptance Criteria). An implementer reading AC5 can
  legitimately record a decision to drop the identifier-safety check,
  satisfying the criterion while violating an explicit requirement and
  reintroducing unquoted control characters and leading `---`/`#` into a
  work item's YAML frontmatter. List all four capabilities under the
  requirement and state that this one's fate is already decided.
- **major / high** — *Whether the jira and linear bash scripts and suites
  are deleted here is never stated, and AC1 assumes suites that other
  sections retire* (Summary / Requirements / Acceptance Criteria). The
  single largest deletion sits in Assumptions rather than Requirements, and
  AC1's verification points at an artefact whose existence the same
  document withdraws.
- **major / medium** — *Bare numeric codes have no named namespace, and the
  sentence calling them "status" contradicts their likely origin*
  (Requirements). 34 is not a valid HTTP status, so "status" misdirects; an
  implementer could key the mapping off the wrong input domain.
- **minor / high** — *"The rule they encode" is never stated, and "provable
  sets" / "provably pre-transmission" are undefined* (Requirements). A
  reviewer cannot judge whether a divergence found during porting is
  intended conservatism or a table bug.
- **minor / high** — *"The tagged, network-touching filter" is referenced
  before anything named it* (Requirements). Leaves AC7's no-network claim
  without a named mechanism.
- **minor / medium** — *Cluster of unresolved referents: "the seam", "the
  value returned", "them", "in step"* (Requirements).
- **minor / medium** — *Two callers are mapped onto three Rust paths
  without saying which discharges which* (Requirements). The guard whose
  loss means overwriting uncommitted work is the part left implicit.
- **suggestion / medium** — *Unnamed actor for taxonomy ownership, and "a
  dispatch token" singular for two new commands* (Requirements /
  Dependencies).

### Completeness

**Summary**: An unusually densely populated work item: every template
section is present and substantively filled, the Requirements enumerate
named files, counts and specific behaviours, and the Acceptance Criteria
run to sixteen bullets including several in Given/When/Then form.
Frontmatter is complete and valid for a story, with the omit-when-empty
linkage slots correctly dropped. The completeness gaps that remain are
asymmetries rather than absences — the integration-skill half of the
cutover is never enumerated or asserted the way the work-script half is,
two substantial requirements have no corresponding acceptance criterion,
and Open Questions is thin relative to the number of decisions the
Requirements explicitly defer.

**Strengths**:

- Every section of the template is present and substantively populated,
  including the optional Technical Notes and Drafting Notes.
- Requirements are exceptionally concrete — named crates, files with line
  references, exact counts — so an implementer can start without follow-up
  questions on the work-script cutover.
- Several criteria use Given/When/Then form and pin observable outcomes
  rather than activities.
- Frontmatter is complete and correct for a story, with empty slots
  correctly omitted.
- Dependencies, Assumptions and Drafting Notes are richly populated,
  including a rejected alternative and its reasoning.

**Findings**:

- **major / high** — *Integration-skill retirement is never enumerated or
  asserted, unlike the work-script cutover* (Requirements / Acceptance
  Criteria). No requirement deletes the 22 Jira and 12 Linear production
  scripts or their suites, none repoints the jira/linear `SKILL.md` bodies
  (only `allowed-tools`), and no criterion asserts the directories are
  gone — while AC1 verifies against a test surface no requirement
  establishes.
- **major / high** — *Two substantive requirements have no corresponding
  acceptance criterion* (Acceptance Criteria). The four per-operation
  exit-code tables and the timeout/page-cap bounding — the two most likely
  to be silently under-implemented — have nothing defining when they are
  done.
- **minor / medium** — *Open Questions is sparse relative to the decisions
  the Requirements defer* (Open Questions). The four capability fates and
  the `EXIT_CODES.md` siting sit inside prose; the one recorded question
  has no owner or deadline.
- **minor / medium** — *Status remains `draft` despite the item declaring
  itself startable whole* (Frontmatter: status). The note that 0194's
  status "is not yet updated" also contradicts Dependencies calling it
  complete.
- **suggestion / medium** — *Story context describes the code surface, not
  whose need is met* (Context).

### Dependency

**Summary**: Dependency capture is unusually strong on the intra-repo,
item-to-item axis: it inherits and enumerates 0194's four cutover
obligations, records the frozen 0204 port surface in detail, and negotiates
an explicit split of suite-floor and `SHELL_LIBRARIES` ownership with 0174.
The gaps are on the axes that leave the repository: the two external
trackers the whole story is built on are never named in Dependencies with
their rate-limit and availability implications, and the credentialed tenant
the contract criteria require is an undischarged prerequisite behind an
unresolved Open Question — yet Dependencies asserts every blocker is
discharged. The machine-readable graph is thinner than the prose: 0204 is
absent from frontmatter, 0174 appears as a relation rather than a
downstream block, and 0194's own record still reads `ready`.

**Strengths**:

- The four obligations inherited from 0194's unfinished cutover are
  enumerated explicitly.
- The ownership boundary with 0174 is negotiated in both directions and
  dated, closing a real ordering hazard.
- The eleven Rust tests resolving paths under `skills/work/scripts/` are
  identified as a build-breaking coupling, forced into the same change.
- The four port-less bridge capabilities are carried forward from 0204 as
  an explicit decide-or-drop obligation.
- The substantive shape of the frozen port is recorded, including the guard
  gap 0204 handed over.

**Findings**:

- **major / high** — *Dependencies claims every blocker is discharged while
  a credentialed Jira/Linear tenant remains unprovisioned* (Dependencies).
  The item reads as schedulable, so it can be driven to its final gate
  before anyone realises the tenant, tokens and secret-hosting decision do
  not exist.
- **major / high** — *Jira REST and Linear GraphQL are absent from
  Dependencies as external systems* (Dependencies). The consumer that
  actually calls them records less than the port that only describes them;
  two criteria can fail for reasons outside the change.
- **major / high** — *0174 is recorded as a relation, not a downstream
  block, and the graph edge disagrees between the two items*
  (Dependencies). A tracker sync trusting 0171's frontmatter drops the
  dependant.
- **major / high** — *0194 is asserted complete but its own record still
  reads `ready`* (Dependencies). The prerequisite the cutover half rests on
  is, on the record, not done.
- **minor / high** — *0204, the most-cited upstream, is absent from the
  frontmatter dependency fields* (Frontmatter: relates_to). 0204's own
  Dependencies cross-references a `blocked_by` field this item does not
  have.
- **minor / medium** — *Ordering between the client half and the cutover
  half is not stated* (Requirements). Deleting the bash surface before
  projection fidelity is proven loses the only oracle.
- **minor / medium** — *New third-party crate ingest is not captured as a
  coupling to the Rust policy lanes* (Technical Notes). `wiremock-rs` and
  rustls must clear `deny.toml`'s licence and advisory policy.
- **minor / medium** — *Two new dispatched sub-binaries couple to the
  release and attribution pipeline* (Dependencies). 0165's signed manifest
  and upload set, and 0203's attribution artefact, are unreferenced.
- **minor / medium** — *The 0170 relation understates the repoint's
  dependency on its command surface* (Dependencies).

### Scope

**Summary**: Internally coherent in theme (finish the Rust migration of the
tracker-facing surface) but not one unit of delivery: two independent
provider client crates plus two binaries, a three-cluster bash cutover, an
eleven-test fixture relocation and conversion, a new conversational flow in
a SKILL, four unresolved design decisions, and the provisioning of
credentialed external targets. The Summary itself joins the halves with
"Then perform the work-item cutover", and 22 requirement bullets support 17
criteria — a profile matching 0136's epic-level children far more than a
story. 0136 records two precedents for splitting exactly this shape (0169
on 2026-07-31, 0173 on 2026-08-05), and the same signature — each
enrichment pass discovering material the previous scope missed — is visible
in this item's own Drafting Notes.

**Strengths**:

- Boundaries with 0174 are stated with unusual precision and allocated
  entry by entry.
- The item refuses to defer residue and enumerates its work-cluster
  deletion set exhaustively.
- Requirements and criteria track each other closely, so both sections
  describe the same scope.
- Drafting Notes carry a real decision log including a rejected
  alternative and its reasoning.

**Findings**:

- **critical / high** — *Two provider integrations and a large cutover
  bundled into one story* (Summary). Three separately deliverable and
  revertible efforts share one review, branch and `done` gate; a defect in
  the Linear client blocks the Jira client and the entire bash retirement.
  Splitting does not recreate the dead-window the 2026-08-10 note rejected,
  because the bash path stays live until the cutover child lands.
- **major / high** — *Declared kind `story` is inappropriate for epic-scale
  scope* (Frontmatter: kind). The documented pattern is that each editing
  pass at this size introduces as many defects as it fixes.
- **major / medium** — *Cutover half itself bundles three independent
  retirement clusters* (Requirements). The clusters depend on different
  prerequisites and share no technical coupling.
- **major / medium** — *Conversational conflict flow is a separate
  user-facing increment* (Requirements). Its dependency is already
  discharged; it shares no crate or test lane with the rest.
- **major / medium** — *Four undecided capability fates leave the delivered
  scope open-ended* (Requirements). "Carry it as an additive port item"
  would reopen the frozen port and ripple across four crates.
- **minor / medium** — *Credentialed tracker provisioning spans a different
  ownership domain* (Open Questions). The unresolved CI-versus-local answer
  silently adds or removes CI workflow work.
- **minor / medium** — *Integrations-side scope stated by reference while
  work-side is enumerated* (Requirements). Roughly half the deletion and
  reimplementation surface has no enumerated in-scope set.

### Testability

**Summary**: The cutover half is exceptionally testable — mechanically
checkable criteria leave a verifier almost nothing to interpret. The
client-building half is not: the largest deliverable rests on an oracle
this same story deletes, and three of the most failure-prone requirements —
the four non-nested exit-code tables, the per-request timeouts and page
cap, and the identifier-safety rejection — have no acceptance criterion at
all, while AC5 actively permits dropping a behaviour the Requirements
mandate. The credentialed-target criteria are unrunnable until the Open
Question about secrets is closed.

**Strengths**:

- Cutover criteria are mechanically decidable single-command pass/fail
  checks.
- Enumeration replaces unbounded language where it matters, so "all" has a
  closed, countable scope.
- Requirements supply a genuine differentiating oracle for the
  highest-risk fidelity concern, with the consequence stated.
- Three criteria use precondition/action/outcome framing well, including
  the `71`-as-well-as-`4` detail.
- The negative criterion about Python and the test lane is verifiable as
  stated.
- Doc-comment updates are pinned to named sites.

**Findings**:

- **critical / high** — *Primary client criterion names an oracle the same
  story deletes* (Acceptance Criteria, first). The largest deliverable can
  be claimed as met by any implementation that exposes the eight subcommand
  names. Replace with per-flow `wiremock` criteria asserting outgoing
  request and parsed response against fixtures captured from today's bash
  flow, and name ADF↔markdown explicitly.
- **critical / high** — *Four exit-code tables and the timeout/page-cap
  bounds have no acceptance criterion* (Requirements vs Acceptance
  Criteria). A misclassified code retries an unretryable mutation; an
  unbounded read path hangs `/list-work-items`. Both ship with every
  criterion green.
- **major / high** — *AC permits dropping the identifier-safety behaviour
  the Requirements mandate* (Acceptance Criteria, fifth). Losing the check
  writes tracker-controlled bytes unquoted into YAML frontmatter.
- **major / high** — *Capability-gap criterion is self-certifying — any
  outcome passes* (Acceptance Criteria, fifth). The one criterion guarding
  against a user-visible regression cannot fail.
- **major / high** — *Contract-run criterion has no defined execution
  procedure and names only two of four operations* (Acceptance Criteria;
  Open Questions).
- **major / medium** — *No procedure for making remote records "match" the
  corpus, and the dangerous case is not pinned* (Acceptance Criteria,
  baseline corpus).
- **major / medium** — *Conflict-rendering criterion relies on a subjective
  threshold with no stated verification lane* (Acceptance Criteria;
  Requirements).
- **minor / high** — *"No duplication of API logic" has no pass/fail
  procedure* (Acceptance Criteria, second).
- **minor / medium** — *"Assertions unchanged in substance" is arguable*
  (Acceptance Criteria, eleven shellout tests).
- **minor / medium** — *Three cleanup requirements have no criterion beyond
  the catch-all* (Requirements vs Acceptance Criteria):
  `_EXPECTED_INTEGRATIONS_SUITES`, the eight `SHELL_LIBRARIES` entries, and
  the cross-skill `jq`/`curl` audit.

## Re-Review (Pass 2) — 2026-08-17

**Verdict:** REVISE

All five lenses re-ran against the revised item. All three criticals cleared;
the verdict holds on major count (threshold: 2), not on severity. The scope
critical was declined by the author and downgraded by the lens itself to major
on re-read, with its recommendation restated once.

### Previously Identified Issues

- 🔴 → ✅ **Clarity/Completeness/Testability/Scope**: integration half
  unspecified, AC1's oracle deleted — **Resolved**. Cluster retirement is a
  requirement with an enumerated inventory; AC1 replaced by per-flow `wiremock`
  fixtures captured pre-deletion.
- 🔴 → ✅ **Testability/Completeness**: four exit-code tables and the
  timeout/page-cap bounds uncovered — **Resolved**, then hardened in pass 2
  (transcribed fixture, two-sided timeout window).
- 🔴 → 🟡 **Scope**: three efforts in one story — **Still present, declined**.
  Downgraded to major. Partially mitigated by the new `## Increments` section
  naming four ordered, individually mergeable changes.
- 🟡 → ✅ **Clarity/Testability**: identifier-safety mandated then droppable —
  **Resolved**. Its own four-case criterion; the capability decision now covers
  three.
- 🟡 → ✅ **Testability/Scope**: self-certifying capability criterion —
  **Resolved**. Outcome-conditional behavioural checks, and the drop branch must
  state an observable replacement.
- 🟡 → ✅ **Dependency/Testability**: credentialed gate unprovisioned and
  narrow — **Resolved**. Prerequisite recorded, all four operations covered,
  execution route required.
- 🟡 → ✅ **Dependency**: Jira REST / Linear GraphQL absent; 0174 edge
  one-directional; 0170 understated — **Resolved** in all three.
- 🟡 → ✅ **Dependency**: 0194 asserted complete but reads `ready` —
  **Resolved differently than recommended**. Verified that commit `c03f2448c6`
  is not an ancestor of this workspace's working copy, so the record is
  divergent rather than stale. Dependencies now directs confirmation against
  0194's artefacts, not its status field.
- 🟡 → ✅ **Testability**: corpus criterion unexecutable; conflict rendering
  subjective — **Resolved**. Seeding procedure, absent-description case, six
  enumerated fields, and a static plus manual verification split.
- 🟡 → ✅ **Clarity**: exit-code namespace unnamed — **Resolved**. Keyed by
  `curl` transport exit code, with the retry rule stated and the key domain
  flagged for confirmation while porting.
- 🔵 → ✅ Minors on referents, the network-lane filter, the dirty-guard
  pairing, taxonomy ownership, Open Questions density, and Context framing —
  **Resolved**.

### New Issues Introduced

Pass 2 surfaced fifteen findings, no criticals. Twelve were fixed in the same
sitting; three remain.

Fixed:

- 🟡 **Testability**: the `jq`/`curl` audit criterion was tautological — any
  surviving entry could be declared 0174's. Now an equality assertion against
  0174's fourteen.
- 🟡 **Testability**: "no longer hold any *migrated* production script" was
  self-referentially escapable. Now `ls … matches nothing`.
- 🟡 **Testability**: "every code in all four bash mapping tables" had no
  oracle once the tables were deleted. Now transcribed verbatim to a committed
  fixture pre-deletion, with a class assertion required per row.
- 🟡 **Testability**: no criterion covered the three work `SKILL.md` bodies
  being repointed. Added as a grep-shaped criterion.
- 🟡 **Testability**: `~35s`/`~65s` gave no pass boundary and implied ~100s of
  wall clock. Now a two-sided window plus a construction-time override.
- 🟡 **Completeness**: registration, release-manifest and `deny.toml`
  obligations appeared only in Dependencies. Now a requirement and a criterion.
- 🟡 **Completeness**: nothing verified the two binaries' own CLI surface — the
  layer the repointed skills invoke. Criterion added.
- 🟡 **Completeness**: Requirements said provisioning was a prerequisite while
  Drafting Notes said "during implementation". Aligned on prerequisite.
- 🟡 **Dependency/Scope**: the additive-port-item branch was filed as a
  dependency on 0204, which is done and frozen against exactly that. Now out of
  scope — a new blocking item instead.
- 🔵 Decisions preamble contradicted its own contents; two undefined states.
  Reworded, states defined.
- 🔵 "Drop the work suite floor to zero" contradicted "removed rather than
  decremented" in the next sentence.
- 🔵 Fixture-deletion and assertion-baseline escape clauses; ADF node-type
  inventory; `RemoteTimestamp` blank/null rule; doc-comment criterion pointing
  at two files this story deletes; ADF/JQL expanded on first use; "the caller"
  and "the dispatcher" named; negative-space criteria given mechanical checks.

Still present:

- 🟡 **Scope**: the item remains four concerns in one `story` (26 requirements,
  22 criteria) with `kind: story`. Declined by the author; `## Increments`
  mitigates the delivery risk without changing the work-item boundary.
- 🔵 **Scope**: several criteria are discharged by editing this document
  (`## Decisions`) rather than by shipped behaviour. Two durable ones — the
  contract-lane execution route and any port-less re-siting — are flagged in
  `## Decisions` as belonging in `tasks/README.md` or an ADR, but not yet moved.
- 🔵 **Dependency**: 0165 and 0203 are recorded in prose but not in
  `relates_to`; `blocked_by` stays empty by design, with the five declared
  upstreams enumerated in Dependencies instead.

### Assessment

The item is materially stronger than at pass 1: every criterion that could not
fail now can, and the three oracles that existed only inside the code being
deleted are captured before deletion. It is **not yet ready for planning**, for
reasons that are now explicit rather than hidden — three Open Questions and two
size-bounding assumptions must close before pickup, and `status` stays `draft`
until they do. The residual scope concern is a recorded, deliberate decision
rather than an unaddressed finding.

## Re-Review (Pass 3) — 2026-08-17

**Verdict:** REVISE

All five lenses re-ran. No criticals; nineteen findings, of which nine are major.
The verdict holds on major count. The defining result of this pass: the
`## Increments` section added in pass 2 to mitigate the scope finding is now the
single largest source of findings, and one of its defects is a live functional
break.

### Previously Identified Issues

- 🟡 **Testability**: tautological `jq`/`curl` audit — **Resolved as a
  criterion, reopened as an inconsistency**. The criterion now asserts set
  equality against 0174's fourteen, but the requirement still says "no skill
  outside this story's set", so the two sections state opposite expected
  outcomes (zero versus fourteen).
- 🟡 **Testability**: "any migrated production script" escapable — **Resolved**.
- 🟡 **Testability**: exit-code tables without an oracle — **Resolved**.
- 🟡 **Testability**: work `SKILL.md` repointing uncovered — **Resolved**.
- 🟡 **Testability**: `~35s`/`~65s` unbounded — **Partially resolved, newly
  self-contradictory**. The two-sided window (30–40s, 60–70s) and the
  sub-second override cannot both hold as written; flagged by clarity and
  testability independently.
- 🟡 **Completeness**: registration and release obligations uncovered —
  **Resolved**.
- 🟡 **Completeness**: binaries' CLI surface uncovered — **Partially
  resolved**. A criterion exists, but "its documented exit code" names no
  document of record, and nothing pins the stdout the repointed skills consume.
- 🟡 **Completeness**: provisioning contradiction — **Resolved**.
- 🟡 **Dependency/Scope**: additive-port-item filed against frozen 0204 —
  **Resolved**. Now out of scope, becoming a new blocking item.
- 🔵 Decisions preamble, floor-to-zero contradiction, fixture-deletion and
  drop-decision escape clauses, ADF inventory, `RemoteTimestamp` rule,
  acronyms, named referents, negative-space checks — **Resolved**, bar the
  Decisions state list (see below).
- 🟡 **Scope**: four concerns in one `story` — **Still present, declined
  again**. Stated once per instruction. The lens notes this is now two REVISE
  passes deep on the trajectory 0136 recorded for 0169 before splitting it.

### New Issues Introduced

Nine majors, seven of them attributable to the pass-2 revision.

Introduced by `## Increments`:

- 🟡 **Dependency**: Increment 2 deletes `work-item-sync-label.sh` while two of
  its live callers — `linear-create-flow.sh:304` and
  `jira-resolve-fields.sh:140` — survive until increment 3. A merged increment 2
  breaks the Jira field-resolution and Linear create flows. The criterion
  asserting those references "vanish with their own files" is only true if both
  clusters die in one change, which the increments forbid.
- 🟡 **Dependency/Completeness/Scope**: the composition-root wiring is assigned
  to no increment, yet increment 2 repoints the work skills at a binary whose
  composition root may still resolve 0194's fakes.
- 🟡 **Clarity**: the Requirements' four "in the same change" clauses contradict
  a four-increment plan, and two "that same change" referents now dangle.
- 🟡 **Scope**: no increment carries exit criteria and no criterion is
  attributed to one, so "individually mergeable" is asserted, not verifiable.
- 🟡 **Scope**: `deny.toml` clearance sits in increment 3 although both
  dependency trees arrive in increment 1.
- 🟡 **Dependency**: the credentialed-target prerequisite is timed to "before
  the first deletion", but increment 1 already requires the harness green
  against real clients — it gates increment 1, and has no named owner.
- 🟡 **Scope**: increment 4 (the conflict flow) is orthogonal by the item's own
  statement, fixes live user-visible degradation, and now inherits three Open
  Questions and a credentialed-target gate it does not need.

Independent of Increments:

- 🟡 **Testability**: the contract-run execution-route criterion is discharged
  by recording *either* answer — writing "a broken client can reach `main`"
  passes it. Same shape on the fixture-capture-source and copyleft records.
- 🟡 **Testability**: "consumer set swept to completion" names no query and
  explicitly excludes the only two references it cites.
- 🟡 **Testability**: no criterion pins the two binaries' stdout against the
  bash output the repointed `SKILL.md` bodies consumed.
- 🟡 **Testability**: the absent-description projection rule — the item's
  highest-stated risk — is verified only in the credentialed lane, which
  Dependencies records as unprovisioned.
- 🟡 **Clarity**: "all four port operations" is followed by three operations
  plus a rule, so a dropped `show` could satisfy the list.
- 🔵 Minors: Decisions declares two states but uses three and omits two records
  its criteria require; `tracker` attributed to both 0204 and 0194;
  `work.integration` names and the read-never-`Terminal` rule undefined; two
  different "fourteen"s; `linear-graphql.sh` classified inconsistently; Context
  overstates 0194 and the prerequisite count; "over" ambiguous in the
  dirty-guard pairing.

### Assessment

The criteria set is materially stronger than at pass 1 and most individual
findings are shallower. But the pass-2 revision introduced seven majors while
resolving eleven, and the mechanism is legible: `## Increments` added a second
ordering model to a document whose Requirements already encoded atomicity, and
the two now contradict each other in ways that produce a real functional break.

That is the pattern 0136 recorded for 0169 — "three editing passes had not
reduced its major-finding count … the signature of one work item carrying more
than a single document can hold consistently." Pass 1: 3 critical, 12 major.
Pass 2: 0 critical, 15 findings. Pass 3: 0 critical, 9 major. The severity
ceiling is falling; the major count is not converging.

Recommendation: rather than a fourth editing pass, either reify the four
increments as child work items (the scope lens's suggestion — 0166 already
parents 0178/0179/0180, so the epic has precedent for a child with children),
or delete `## Increments` and restore the single-change atomicity the
Requirements state. Both remove the contradiction at its source; further
in-place editing is likely to trade one set of majors for another.

## Acceptance — 2026-08-17

**Accepted.** Verdict changed from REVISE to APPROVE by Toby Clemson, and the
target work item moved to `ready`.

The open findings recorded above were **accepted rather than resolved**. They are
not withdrawn and remain the record of what is known to be imperfect; they carry
into planning rather than blocking it. Specifically still open at acceptance:

- The three Open Questions on 0171 — the credentialed target's secrets siting, the
  fate of the three port-less bridge capabilities, and the `EXIT_CODES.md` siting —
  plus the two ⚠️ size-bounding assumptions in 0211 and 0212.
- Three self-contradictions introduced during the fix rounds: 0211's mock-server
  deletion being simultaneously unconditional and deferrable, 0211's `jq`/`curl`
  requirement asserting a survivor set its own criterion says the child cannot
  reach, and 0213's stub-on-`PATH` seam defeating the one predicate item that would
  catch a malformed `--resolve` template.
- Two latent gaps better closed by implementation than by further specification:
  the non-port provider surface (five of eight flows) being owned by neither 0210
  nor 0211, and 0210 carrying no criterion for HTTP-status or GraphQL error
  classification or auth.

Rationale for accepting rather than iterating: across four review passes the
severity ceiling fell (3 criticals → 1 → 0 → 0) while the major count did not
converge, and each fix round introduced two or three new majors of one shape — a
requirement updated without its criteria, or the reverse. The two correctness traps
that mattered (the unowned port-less capabilities, and the `work-item-sync-label.sh`
ordering break) are both closed. Further speculative specification was judged less
valuable than starting 0210 and discovering the real shape of the client crates.
