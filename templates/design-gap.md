---
date: "{ISO timestamp}"
type: design-gap
current_inventory: "{path to current inventory.md}"
target_inventory: "{path to target inventory.md}"
author: "{author name}"
status: draft
tags: [design, gap-analysis]
---

# Design Gap Analysis: {current-source} → {target-source}

## Overview

[One paragraph framing what was compared, when, and at what fidelity.
Note any limitations: partial inventories, auth-walled areas excluded, etc.]

## Token Drift

[Intro paragraph framing the category and why the drift matters.
Each entry below is written as actionable prose so that `extract-work-items`
can detect it via its cue-phrase contract ("we need to…", "the system must…",
"users need…", "implement X to…").]

[For each token gap: one prose paragraph naming the drift, the impact, and
what needs to change. Example: "The colour palette in the current app uses 14
distinct hues that do not map onto the prototype's 8-token scale. We need to
migrate the codebase to use the prototype's named tokens… affecting every
component that currently hardcodes hex values. Refs: §Design System / Tokens
in both inventories."]

## Component Drift

[Intro paragraph.]

[For each component with divergent variants, props, or behaviour: one prose
paragraph per gap. Each paragraph should name the component, describe the
drift, and express the change as actionable language.]

## Screen Drift

[Intro paragraph — visual and structural differences within screens that exist
on both sides.]

[For each screen-level gap: one prose paragraph per divergence.]

## Net-New Features

[Intro paragraph — capabilities present in the target but absent in the current.]

[For each net-new feature: one prose paragraph naming the feature, the screens
it surfaces on, and what needs to be implemented.]

## Removed Features

[Intro paragraph — capabilities present in the current but absent in the target.
Each entry should ask for explicit confirmation before recommending removal, as
the absence may be intentional (scope cut) or an oversight in the target design.]

[For each potentially removed feature: one prose paragraph naming the feature
and asking whether its removal is intentional.]

## Suggested Sequencing

[Optional: recommended implementation order based on dependency analysis from
the token, component, and feature sections. Written as prose, not a checklist,
so it reads as guidance rather than a locked plan.]

## References

- Current inventory: `{path to current inventory.md}`
- Target inventory: `{path to target inventory.md}`
- Related: [research docs, ADRs, plans]
