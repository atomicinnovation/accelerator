---
date: "{ISO timestamp}"
type: design-inventory
source: "{source-id}"
source_kind: "{code-repo | prototype | running-app}"
source_location: "{path or URL}"
git_commit: "{commit hash — omit if not a code repo}"
branch: "{branch — omit if not a code repo}"
crawler: "{code | runtime | hybrid}"
author: "{author name}"
status: draft
sequence: 1
screenshots_incomplete: false
tags: [design, inventory, "{source-id}"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author name}"
---

# Design Inventory: {source-id}

## Overview

[Scope: which routes/areas covered, which excluded. Crawler methodology used.
Known gaps: auth-gated areas, dynamic content not reached, intentional exclusions.]

## Design System

### Tokens

[Tables of `name: value` for colours, typography, spacing, radii, shadows, motion.
Source file:line refs where available.]

| Token | Value | Category |
|-------|-------|----------|
| | | |

### Layout Primitives

[Grid system, container widths, breakpoints, z-index scale.]

## Component Catalogue

### {ComponentName}

- **Variants / props**: [list]
- **Used on screens**: [{screen-id}, ...]
- **Source**: `file:line` (for code) or selector path (for runtime)

[Repeat per component]

## Screen Inventory

### {screen-id} — {route or URL}

- **Purpose**: [one line]
- **Components used**: [{ComponentName}, ...]
- **States observed**: loading | empty | error | success | partial
- **Key interactions**: [click → outcome]
- **Screenshot**: `screenshots/{screen-id}.png` (if Playwright was used)

[Repeat per screen]

## Feature Catalogue

### {feature-id}

- **Capability**: [one sentence, screen-independent]
- **Surfaces on**: [{screen-id}, ...]
- **Depends on**: [{external API, state slice, ...}]

[Repeat per feature]

## Information Architecture

[Route table or navigation graph — textual, or Mermaid diagram.]

## Crawl Notes

[Anything that surprised the crawler: dead-ends, auth walls, dynamic content gaps,
cap or timeout hits, routes not reached.]

## References

- Source: `{path or URL}`
- Related: [inventory paths, ADRs, research docs]
