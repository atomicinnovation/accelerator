---
title: Design Convergence
---

Design convergence skills capture two design surfaces — a current
frontend and a target prototype — as structured inventory artefacts,
then compute a structured gap between them. The gap artefact's prose
paragraphs satisfy the cue-phrase contract that
[`extract-work-items`](../reference/skills/work/extract-work-items.md)
consumes, so the workflow plugs straight into the existing work-item
lifecycle.

```
inventory-design (current)  ─┐
                             ├─▶ analyse-design-gaps ─▶ extract-work-items ─▶ meta/work/*
inventory-design (target)   ─┘
```

- [`inventory-design`](../reference/skills/design/inventory-design.md)
  generates a structured design inventory for one frontend source —
  tokens, components, screens, and features — by crawling it with
  static code analysis, live Playwright inspection via the
  [browser agents](../reference/agents.md#browser-agents), or both
  (`--crawler code|runtime|hybrid`). Each snapshot is self-contained
  (markdown plus screenshots in a dated directory under
  `meta/research/design-inventories/`); re-running for the same source
  supersedes the prior snapshot without losing it.
- [`analyse-design-gaps`](../reference/skills/design/analyse-design-gaps.md)
  compares two inventories and emits a gap artefact under
  `meta/research/design-gaps/`.

The three-step flow, end to end:

```
/inventory-design current ./apps/webapp
/inventory-design prototype https://prototype.example.com
/analyse-design-gaps current prototype
/extract-work-items <gap-file>
```

## Runtime requirements

The `runtime` and `hybrid` crawler modes drive Playwright's Chromium
through a local executor daemon. They need **Node ≥ 20**, macOS or
Linux, and ~500 MB of disk for the first-run Chromium install (cached
per machine under `~/.cache/accelerator/playwright/`). The `code` mode
needs none of this. If bootstrap fails in `hybrid` mode the skill falls
back to a `code`-only crawl with a printed notice.

Authenticated crawls, security considerations (auth headers, screenshot
masking, side-effecting forms), cache cleanup, and troubleshooting are
covered in full on the
[`inventory-design`](../reference/skills/design/inventory-design.md)
reference page.
