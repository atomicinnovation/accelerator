---
type: work-item
id: "0106"
title: "Invoke Plugin Scripts by Bare Path in Skill Bodies"
date: "2026-06-10T20:58:58+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: task
priority: medium
relates_to: ["work-item:0107"]
source: "issue-research:2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission"
tags: [permissions, allowed-tools, skills, plugin, authoring-convention]
last_updated: "2026-06-10T20:58:58+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0106: Invoke Plugin Scripts by Bare Path in Skill Bodies

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Skills repeatedly prompt for permission to run `artifact-derive-metadata.sh` and
sibling `artifact-*`/`config-*` scripts because the model wraps the invocation as
`bash <path>`, which escapes the `allowed-tools` prefix rule (`bash` is not a
stripped wrapper, so the authorized path becomes a mere argument). Add an explicit
"run directly as an executable, never prefix with `bash`/`sh`/`env`" directive at every
`artifact-*`/`config-*` call site, and convert the bare unlabeled code fences in
`create-adr`/`extract-adrs` to inline code to remove the strongest wrapping
amplifier.

## Context

The scripts self-execute (shebang `#!/usr/bin/env bash` + execute bit) and the
bare-path form already matches the existing
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)`/`config-*` rules — proven
empirically in the investigation session, where the bare-path invocation completed
silently under the same active rule that the `bash`-prefixed form prompts against.
The only gap is that skill bodies state *what* to run but never *how*, leaving the
invocation shape to model discretion — and the dominant training prior for "run a
`.sh` file" is `bash script.sh`. ("Stripped wrapper" here means a process wrapper
the permission matcher removes before comparing the command against a rule — only
`timeout`/`time`/`nice`/`nohup`/`stdbuf` are stripped, and `bash` is not, so a
`bash`-prefixed command no longer begins with the authorized path.) This item closes
the gap at the source — adding the bare-path directive at each call site and
converting the two bare ADR fences to inline code.

## Requirements

- Establish the canonical set of affected call sites as the **first step** of the
  task by enumerating every *occurrence* (not merely every file) where a skill body
  invokes an `artifact-*`/`config-*` script. A "call site" is one such passage; a
  single `SKILL.md` may contain several, and each requires its own directive. The
  authoritative set is the output of:
  `grep -rn 'scripts/\(artifact\|config\)-' --include=SKILL.md skills/`
  (run from the plugin root; `-r --include=SKILL.md` matches every `SKILL.md` at any
  depth without relying on shell globstar, and `-n` lists each occurrence with its
  line, not just the matching file). This grep result — not the illustrative list in
  the source research — is the denominator against which "every call site" is
  verified. A conforming directive must sit in the **same passage** as the occurrence
  it governs (the same step/bullet/paragraph, or immediately adjacent), so a single
  file-level note does not count as covering multiple occurrences.
- Add a short imperative directive at **every** call site in that enumerated set,
  telling the model to run the script by bare path and never prefix it with
  `bash`/`sh`/`env`. A **conforming directive** must, at minimum: (a) name all three
  wrappers `bash`/`sh`/`env` as prohibited, and (b) instruct that the bare path is
  run directly as an executable. Use the following canonical template — adapted from
  the source research's Recommended Fix blockquote to add `env` (the research names
  only `bash`/`sh`); this work item standardises on the three-wrapper form:
  > Run the script **directly** as an executable: `<bare-path>`. Do **not** prefix
  > the invocation with `bash`/`sh`/`env` — doing so escapes the skill's
  > `allowed-tools` permission and forces an unnecessary prompt.
- Convert the bare unlabeled ``` ``` ``` code fences housing the script path in
  `create-adr` (`SKILL.md:124-126`) and `extract-adrs` (`SKILL.md:120-122`) to
  inline code. (Line numbers are version-pinned — see Technical Notes; locate the
  fences by content if they have drifted.)
- Make no `allowed-tools` rule changes in this item — rely on the existing shebang
  + execute bit and existing rules.

## Acceptance Criteria

- [ ] Given the canonical call-site set produced by
      `grep -rn 'scripts/\(artifact\|config\)-' --include=SKILL.md skills/`, when each
      occurrence in that set is inspected, then every occurrence carries a conforming
      directive — one that names all three wrappers `bash`/`sh`/`env` as prohibited
      and instructs that the bare path is run directly as an executable — located in
      the same passage as (or immediately adjacent to) that occurrence.
- [ ] Given `create-adr/SKILL.md` and `extract-adrs/SKILL.md`, when the
      script-path passages are inspected, then each file contains a
      backtick-delimited **inline** occurrence of the script path (presence check)
      and **no** bare unlabeled fenced block housing that path remains (absence
      check).
- [ ] Given the full skill set, when the fence-state regression guard is run (a
      line-by-line parser that flags any **unlabeled** fenced block whose body contains
      an `artifact-*`/`config-*` script path, excluding `!`…`` load-time substitutions —
      see the plan's Testing Strategy for the canonical `awk` implementation), then it
      returns **zero** matches. Precondition: the guard must first be confirmed to match
      the pre-conversion bare fences as a known-positive, so an empty result proves
      cleanliness.
- [ ] No `allowed-tools` frontmatter is modified by this item.

## Open Questions

- None blocking for the editing work — scope is confirmed as the body-directive and
  fence-conversion edits only.
- Non-blocking residual risk: is the source research's "first-call-only
  `allowed-tools` enforcement" quirk (Hypothesis 3) present in the target Claude Code
  version? If confirmed, the body directives alone may not eliminate every prompt and
  the issue would need revisiting. A small spike could confirm the quirk's presence.
- The lint/test guardrail that would automate this item's manual grep checks is now
  tracked as work item 0107 (blocked by this item).

## Dependencies

- Blocked by: none
- Blocks: none
- Relates to: 0107 — a plugin lint/test guardrail that cross-checks each script
  invocation in skill bodies against the skill's `allowed-tools` rules, asserting the
  invocation *shape* is covered. It automates the manual grep checks this item relies
  on and prevents the bare-path convention from eroding in later edits (per the source
  research's Prevention section).
- External coupling: the fix's effectiveness depends on Claude Code's permission
  matcher continuing to (a) strip only `timeout`/`time`/`nice`/`nohup`/`stdbuf` as
  recognised wrappers — *not* `bash`/`sh`/`env` — and (b) match the bare path by
  prefix. This behaviour lives in the harness, not this repo; if a future Claude Code
  version changes its wrapper-stripping list or match semantics, the convention is the
  place to re-validate.

## Assumptions

- The existing shebang + execute bit on each `artifact-*`/`config-*` script remain
  in place, so the bare-path form is reliably executable across the affected
  environments. (The change targets the whole `artifact-*`/`config-*` script family,
  not only the `artifact-derive-metadata.sh` used as the running example.)
- "Affected skills" is the full set whose body invokes any `artifact-*`/`config-*`
  script — i.e. every skill matched by the canonical grep in Requirements — not only
  the partial, illustrative list named in the source research.

## Technical Notes

- Highest-risk sites are the bare unlabeled code fences in `create-adr`
  (`SKILL.md:124-126`) and `extract-adrs` (`SKILL.md:120-122`), which read as shell
  snippets and invite reconstruction as `bash <path>`. These line numbers were
  captured against plugin version `accelerator/1.22.0-pre.11`; if working against a
  different version, locate the fences by content (the regression-guard grep does
  this) rather than by line number, which may have drifted.
- Inline-code call sites (e.g. `research-codebase`, `create-plan`, `create-note`,
  `research-issue`, `review-*`, `describe-pr`) are lower-risk but should still carry
  the directive for consistency.
- The `allowed-tools` rule `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` already
  authorizes the bare-path shape; this change keeps body invocation shape and rule
  prefix in lockstep.

## Drafting Notes

- Kind set to `task` (operational editing of plugin skill assets), per scope
  confirmation during enrichment.
- Enumeration confirmed as *all* `artifact-*`/`config-*` call sites (broadest,
  matches root cause), not just the named 15 or just `artifact-derive-metadata.sh`.
- Verification confirmed as static review; the runtime "no prompt" criterion was
  reframed into inspectable conditions rather than a manual reproduction step.
- Scoped strictly to the body-directive and fence-conversion edits; changing
  `allowed-tools` rules is intentionally excluded from this item.
- AC3 was corrected during planning (research §5): the original
  `rg -U '```\n…--pcre2'` regex was removed because it errors without `--pcre2`
  and, with it, over-matches 12 non-target files via inter-fence false positives —
  so it can never return empty and "empty proves cleanliness" was false. It is
  replaced by the fence-state `awk` guard.
- The plan's single fixed canonical directive sentence ("Run the bare path
  **directly** as an executable; never prefix it with `bash`/`sh`/`env` …") is the
  **authoritative** wording for the convention; the Requirements blockquote template
  above is illustrative-only. Both meet the conformance minimum, but only the plan's
  fixed string is reproduced verbatim across sites and matched by 0107's lint.

## References

- Source: `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
