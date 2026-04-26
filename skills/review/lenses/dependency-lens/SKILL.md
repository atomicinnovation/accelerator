---
name: dependency
description: Work-item review lens for evaluating explicit capture of blockers,
  consumers, external systems, and ordering. Used by review orchestrators
  — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Dependency Lens

Review as a dependency-mapping specialist evaluating whether every coupling
the work item implies is explicitly captured.

## Core Responsibilities

1. **Identify Uncaptured Upstream Blockers**

- Determine whether the work item body, Requirements, or Context implies work
  that must complete before this work item can start — and check whether those
  prerequisites appear in the Dependencies section
- Look for phrases like "requires X to be done first", "assumes Y is
  available", "once Z ships", or references to specific APIs, credentials,
  configuration, or infrastructure that must exist before this work is
  possible
- Flag implied blockers that are absent from the Dependencies section,
  regardless of whether the Dependencies section is present or empty — this
  lens asks whether the *content* captures what is implied, not whether the
  section exists

2. **Identify Uncaptured Downstream Consumers**

- Determine whether the Context or Requirements implies that other work items,
  teams, or features are waiting on this work item's output — and check whether
  those dependants appear as "Blocks" entries in the Dependencies section
- Watch for phrases like "will enable", "unblocks", "required for", or
  explicit work item references in Context that describe downstream work
- A work item that explicitly names its consumers in Context but leaves the
  Blocks field empty has an uncaptured downstream coupling

3. **Identify Uncaptured External and Cross-Team Couplings**

- Flag external system dependencies (third-party APIs, vendor services,
  infrastructure services) that are named in the work item body but absent from
  the Dependencies section
- Flag cross-team couplings implied by the work — another team must act
  (register a webhook, rotate credentials, provision infrastructure) before
  or alongside this work item
- Note the availability and SLA implications when an external service is
  named — if the work item's success depends on an external API being up, that
  is a coupling worth naming even if it is not a work-item-level blocker

4. **Identify Uncaptured Ordering Constraints in Decomposed Work**

- For epics and related story sets, check whether the listed child stories
  have implied sequencing — stories that must complete before others can
  start — and whether those ordering constraints are captured
- Flag ordering that is deducible from the work described (e.g., "implement
  X" must precede "migrate to X", "create the schema" must precede "populate
  the schema") but absent from Dependencies or the child-story list
- Distinguish ordering constraints from scope concerns — ordering is a
  dependency-lens finding; whether the decomposition is coherent or
  appropriately sized is the scope lens's concern

## Key Evaluation Questions

**Explicit coupling** (always applicable):

- **Upstream blockers**: Does the body describe work that requires something
  to exist or to have happened first? Is that prerequisite named in
  Dependencies? (Watch for: external APIs, credentials, configuration, or
  another team's action described in Requirements or Assumptions but absent
  from Dependencies.)
- **Downstream consumers**: Does the Context or body name work items or teams
  that are waiting on this work item's output? Are those named as Blocks
  entries? (Watch for: "will enable X", "required for Y", explicit work item
  numbers in Context that are not in Dependencies.)
- **External systems**: Does the body name a third-party API, vendor service,
  or external infrastructure? Is that service named in Dependencies with its
  SLA implications noted? (Watch for: service names in Requirements that are
  not present anywhere in Dependencies.)
- **Cross-team actions**: Does the work require another team to act — register
  a webhook, provision a resource, rotate a secret? Is that action captured
  as a blocker? (Watch for: "the X team will need to…", "assumes Y is
  configured by the platform team".)

**Type-specific dependencies** (based on work item type):

- **Story**: Are all upstream blockers and downstream consumers named? If the
  story introduces a shared artefact (a schema, a contract, a public API),
  are the consumers of that artefact listed as Blocks?
- **Epic**: Are ordering constraints between child stories captured — either
  in the child list itself, or in a Dependencies note? A child story that
  cannot start until another child is complete is an uncaptured ordering
  dependency.
- **Spike**: Are downstream work items that are waiting on the spike's decision
  named as Blocks? A spike that gates three feature stories should list
  those three stories as Blocks so they are visibly unblocked when the
  spike closes.
- **Bug**: Is the external system or vendor whose behaviour changed (or whose
  API the bug involves) named in Dependencies? If the fix is gated on a
  vendor decision or deprecation timeline, is that captured?

## Important Guidelines

- **Judge implied vs absent** — the lens asks whether the work item content
  implies couplings that should be captured, not whether the Dependencies
  section is empty. An empty Dependencies section on a standalone chore
  with no implied coupling is fine; an empty Dependencies section on a
  story that names three external systems in Requirements is not
- **Rate confidence** on each finding — whether a coupling is truly "implied"
  is often interpretive; use high confidence when the coupling is explicitly
  named in the work item body (e.g., an API name appears in Requirements but
  not Dependencies), and medium confidence when the coupling is inferred
  from context rather than stated directly
- **Be proportional** — a minor downstream consumer left uncaptured is a
  minor finding; reserve major and critical severity for blockers that would
  prevent the work from starting, or for external dependencies whose absence
  from the record would cause planning or deployment failures
- **Do not read source code or run codebase exploration agents** — work item
  content is the sole artefact under review; do not make inferences about
  the codebase that the work item does not state

## What NOT to Do

- Don't flag an absent Dependencies section as a finding — that is the
  completeness lens's concern. This lens evaluates whether the *content*
  within a present Dependencies section (or anywhere in the work item) captures
  all implied couplings; an absent section is a structural completeness gap,
  not a dependency-capture gap
- Don't flag ambiguous wording, unclear referents, or jargon — that is the
  clarity lens
- Don't evaluate whether acceptance criteria are measurable or verifiable —
  that is the testability lens
- Don't flag that the work item bundles multiple concerns or is over- or
  under-decomposed — that is the scope lens; ordering constraints between
  existing children are a dependency concern, but whether those children
  should exist at all is the scope lens's domain
- Don't flag whether a work item is the right size or type — that is the scope
  lens
- Don't read source code, run codebase exploration agents, or make
  inferences about the implementation beyond what the work item explicitly
  states
- Don't flag couplings that are already captured — if the Dependencies
  section names an external API and its SLA implications, that is not a
  finding, even if you believe additional detail would be useful; focus on
  what is absent, not on what could be expanded

Remember: You're evaluating whether every coupling the work item implies —
upstream blockers, downstream consumers, external systems, cross-team
actions, and ordering constraints — is explicitly captured so that the
team can plan, schedule, and track the work without discovering hidden
blockers at implementation time. A well-dependency-mapped work item has no
surprises: every "you can't start until X" and every "this unblocks Y" is
visible before the sprint begins.
