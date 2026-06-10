---
type: issue-research
id: "2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission"
title: "Investigation: bash wrapper prefix defeats skill allowed-tools permission for artifact-derive-metadata.sh"
date: "2026-06-10T19:55:06+00:00"
author: "Toby Clemson"
producer: research-issue
status: complete
topic: "Skills repeatedly prompt for permission to run artifact-derive-metadata.sh when invoked as `bash <script>` despite an allowed-tools rule"
tags: [research, debugging, permissions, allowed-tools, skills, plugin]
revision: "faa5abe46ae193f6e329fbaa6868a066defaa824"
repository: "miscellaneous"
last_updated: "2026-06-10T19:55:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Investigation: bash wrapper prefix defeats skill allowed-tools permission for artifact-derive-metadata.sh

**Date**: 2026-06-10 19:55 UTC
**Author**: Toby Clemson
**Git Commit**: faa5abe46ae193f6e329fbaa6868a066defaa824
**Branch**: (jj change `ynkpmkvwpunz`, no bookmark)
**Repository**: miscellaneous (accelerator plugin)

## Issue Description

Skills in the Accelerator plugin frequently prompt the user for permission to run
`scripts/artifact-derive-metadata.sh`, even though every such skill already declares
the script in its `allowed-tools` frontmatter. The prompt appears specifically when
the script is invoked in the wrapped form:

```
bash ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh
```

The script is executable and carries a `#!/usr/bin/env bash` shebang, so the `bash`
wrapper is unnecessary. The reporter's hypothesis: the extra leading `bash` word makes
the invocation escape the `allowed-tools` rule, triggering a prompt that the bare-path
invocation would not.

## Input Classification

Mixed — a behavioral description ("frequently requests permission") paired with a
concrete reproduction string (`bash ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`)
and a stated suspected mechanism (the leading `bash`).

## Affected Components

- `skills/research/research-issue/SKILL.md:7-9` — `allowed-tools` declares
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` and `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)`.
- `skills/**/SKILL.md` body text (15 skills) — all reference the script as a **bare path**:
  `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` (research-codebase, research-issue,
  create-plan, review-plan, validate-plan, create-adr, extract-adrs, create-note, review-pr,
  describe-pr, create-work-item, review-work-item, extract-work-items, …).
- `scripts/artifact-derive-metadata.sh:1` — `#!/usr/bin/env bash`, mode includes execute bit.
- Claude Code permission matcher (harness, not in this repo) — performs the `allowed-tools` match.

## Timeline / Reproduction

1. A skill activates; its `allowed-tools` frontmatter authorizes
   `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)`.
2. The skill body instructs the model to run the metadata script. The body shows the
   **bare path** form, but the rendered instruction the model sees often has
   `${CLAUDE_PLUGIN_ROOT}` already expanded to an absolute path.
3. The model issues the command through the Bash tool. When it prepends `bash ` (a habitual,
   "safe" way to run a script), the command string becomes
   `bash <abs-path>/scripts/artifact-derive-metadata.sh`.
4. The permission matcher compares the command against the rule. The rule's effective
   prefix is `<script-path>/scripts/artifact-…`; the command now begins with the word
   `bash`, so the prefix no longer matches.
5. No allow rule matches → the user is prompted.

Contrast — the bare-path invocation **does not** prompt. Running
`${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` directly during this
investigation completed silently under the same active `allowed-tools` rule, returning
its metadata output. This is the controlled experiment that isolates the `bash` prefix.

## Hypotheses

### Hypothesis 1: The leading `bash` wrapper word breaks the prefix/glob match
- **Evidence for**:
  - `allowed-tools` rules match the literal command string with `*` glob/prefix semantics.
    `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` matches a command **starting with**
    that path; `bash <path>` starts with `bash`, not the path.
  - Claude Code strips a fixed set of *recognised* process wrappers before matching
    (`timeout`, `time`, `nice`, `nohup`, `stdbuf`). **`bash` is not in that list**, so it is
    treated as part of the command, not stripped (per Claude Code permissions documentation).
  - Empirical test: the **bare-path** invocation ran with no prompt under the active rule;
    the reporter observes the **`bash`-prefixed** form prompts. Only the wrapper differs.
  - Every skill body already uses the bare path — the wrapper is introduced at invocation
    time by the model, not by the skill.
- **Evidence against**: None found.
- **Verdict**: **Confirmed** — this is the root cause.

### Hypothesis 2: Literal `${CLAUDE_PLUGIN_ROOT}` in the rule fails to match an expanded path in the command
- **Evidence for**: The rule text contains the literal token `${CLAUDE_PLUGIN_ROOT}`, while
  the model frequently types the fully expanded absolute path; a naive string matcher would
  not equate them.
- **Evidence against**: The bare-path invocation in this session used the **expanded absolute
  path** and was auto-approved with no prompt — proving the plugin loader expands
  `${CLAUDE_PLUGIN_ROOT}` in `allowed-tools` rules before matching, so expanded-path commands
  *do* match. The variable expansion is therefore handled correctly and is not the cause.
- **Verdict**: **Eliminated**.

### Hypothesis 3: Upstream bug — `allowed-tools` only auto-approves the first Bash call per session
- **Evidence for**: Open Claude Code issues report `allowed-tools` granting permission only on
  the first matching Bash call, or being parsed-but-not-enforced (e.g. anthropics/claude-code
  #60515, #14956, #37683).
- **Evidence against**: The reporter's symptom is tied specifically to the `bash <script>` form,
  not to call ordering; the bare-path form is consistently approved. The observed behavior is
  fully explained by Hypothesis 1 without invoking an enforcement bug.
- **Verdict**: **Inconclusive** — a possible aggravating factor in some sessions, but not the
  mechanism behind this report.

## Root Cause

Claude Code's `allowed-tools` Bash matcher matches the command string by prefix/glob against
the rule. The rule `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` authorizes commands that
**begin with** the (expanded) script path. When the model invokes the script as
`bash ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`, the command begins with the
word `bash`. Because `bash` is **not** one of the process wrappers Claude Code strips before
matching (only `timeout`, `time`, `nice`, `nohup`, `stdbuf` are), the wrapper is treated as the
command and the path becomes a mere argument — so the rule's path prefix no longer matches and
the user is prompted. The script's shebang and execute bit make the `bash` wrapper entirely
unnecessary; removing it makes the bare-path invocation match the existing rule.

## Causal Chain

1. Skill body instructs the model to run the metadata script (shown as a bare path).
2. The model, defensively, prepends `bash ` to the path before calling the Bash tool.
3. The command string now starts with `bash`, not with the authorized script path.
4. `bash` is not a stripped wrapper, so the matcher keeps it in the command for comparison.
5. The `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` prefix fails to match `bash …`.
6. No allow rule matches → permission prompt is shown to the user.

## Why the Model Adds the `bash` Prefix

The `allowed-tools` rule and the matcher explain *what* breaks; this section explains *why the
model emits the wrapped form at all*, given that **no skill body ever writes `bash`**. Every
invocation site uses a bare path — either inline code (`Run the …script`, `Invoke \`…\``) or a
fenced block — with the verbs "run", "invoke", "using". The `bash` is entirely model-generated.
These are competing inference priors, not a single switch; they are ranked by estimated influence.

1. **"Run a `.sh` file" has a dominant training prior that is *not* the bare path.** In almost all
   training text, you execute a script file as `bash script.sh`, `./script.sh`, or `sh script.sh`;
   a bare absolute path used as a command is comparatively rare. Told to "run
   `…/artifact-derive-metadata.sh`", the model does not copy the literal string — it *re-derives a
   runnable command*, and the highest-probability completion for "execute this `.sh`" is
   `bash <path>`. The `.sh` extension is the trigger.

2. **Bare unlabeled code fences amplify it.** In `create-adr` (`SKILL.md:124-126`) and
   `extract-adrs` (`SKILL.md:120-122`) the path sits inside a bare ```` ``` ```` block with no
   language. A path inside an unlabeled fence reads as "a shell snippet to run," and snippets get
   *reconstructed as commands* rather than pasted verbatim — re-applying prior (1). These two sites
   are the highest-risk. The inline-code sites (`research-codebase`, `create-plan`, `create-note`,
   `research-issue`, `review-*`, `describe-pr`) read slightly more like "run this exact thing" and
   should wrap less often. This variance matches the reporter's "frequently, not always."

3. **Defensive-execution heuristic.** `bash X` runs even if the execute bit is missing, whereas a
   bare `/path/X.sh` fails with "permission denied" when it is not executable. The model has learned
   `bash X` is the *more robust* invocation, so faced with a path it did not create and cannot verify
   is executable, the "safe" choice is to wrap it. The safety reflex is precisely what defeats the
   permission match.

4. **The tool is named "Bash" and described as "Executes a bash command."** That frame maps "run
   this script" → "emit a bash command," nudging the model toward literally writing the word `bash`.

5. **No counter-instruction.** The bodies specify *what* to run, never *how*. With nothing pulling
   the model back to the bare form, priors (1)–(4) win by default. This is the one lever the plugin
   author fully controls — see Fix Option A.

**Ruled out — external rewrite.** The reporter's global RTK hook rewrites recognised CLIs as
`rtk <cmd>`; it does not inject `bash`, and the permission dialog reflects the model's emitted
string. Confirm in 10 seconds by checking the prompt shows exactly `bash ${CLAUDE_PLUGIN_ROOT}/…`
(model prior) and not `rtk bash …` (a rewrite). All evidence points to the model prior.

## Contributing Factors

- The rendered skill instruction often presents the **already-expanded** absolute path, which
  reads like an ordinary file path and nudges the model toward `bash <path>` habits.
- Skill bodies state *which* script to run but do not state *how* to invoke it (bare vs wrapped),
  leaving the choice to model discretion (see "Why the Model Adds the `bash` Prefix" above).
- Bare unlabeled code fences at the `create-adr` and `extract-adrs` sites are the strongest
  amplifier of the wrapping prior; inline-code sites are lower-risk.
- `allowed-tools` authorizes only the bare-path shape, so any equivalent-but-differently-shaped
  invocation (`bash …`, `sh …`, `env …`) silently falls outside the rule.

## Fix Options

| Option | Description | Risk | Effort |
|--------|-------------|------|--------|
| A | In each skill body, instruct the model to run the script **directly as an executable** (bare path) and **never** prefix it with `bash`/`sh`; and convert the bare unlabeled code fences at the `create-adr`/`extract-adrs` sites to inline code to remove the strongest wrapping amplifier (see "Why the Model Adds the `bash` Prefix"). Relies on the existing shebang + execute bit and the existing `artifact-*`/`config-*` rules. | Low | Low |
| B | Broaden `allowed-tools` to also authorize the wrapped forms, e.g. add `Bash(bash ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)` (and `config-*`, and optionally `sh …`). Belt-and-suspenders alongside A. | Low | Med (touches ~15 skills) |
| C | User-side stopgap: add an allow rule to `~/.claude/settings.json`, e.g. `Bash(bash /Users/tobyclemson/.claude/plugins/cache/*/scripts/artifact-*)`, requiring no plugin edit. | Low | Low |

## Recommended Fix

**Option A as the primary fix, reinforced by Option B.** A addresses the cause at its source:
the script is designed to self-execute (shebang + execute bit), the bare-path form already
matches the existing rule (proven empirically this session), and every skill body already
uses the bare path — the only gap is an explicit directive telling the model not to wrap it.
Add a short, imperative line at each invocation site, e.g.:

> Run the script **directly** as an executable: `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`.
> Do **not** prefix the invocation with `bash` or `sh` — doing so escapes the skill's
> `allowed-tools` permission and forces an unnecessary prompt.

Layer Option B on top for robustness against model drift: extend each skill's `allowed-tools`
to also cover the wrapped form so an accidental `bash …` still matches. Option C is a reasonable
immediate workaround for the user before a new plugin version ships, but it is per-machine and
sensitive to the version segment in the cache path, so it should not be the long-term fix.

## Prevention

- Adopt a skill-authoring convention: plugin scripts are invoked by **bare path** (executable +
  shebang), never via a `bash`/`sh`/`env` wrapper, and the body says so explicitly at each call site.
- Avoid presenting a runnable script path inside a bare unlabeled ```` ``` ```` fence — it reads as a
  shell snippet and invites the model to reconstruct it as `bash <path>`. Prefer inline code plus an
  imperative "run directly" instruction.
- Add a plugin lint/test that cross-checks every script invocation in skill bodies against the
  skill's `allowed-tools` rules — asserting the exact invocation *shape* used in the body is
  covered by a rule (catches wrapper/quoting/path-shape mismatches before release).
- Prefer authorizing scripts at the directory/glob level the body actually uses, and keep the
  body's invocation shape and the rule's prefix in lockstep.

## Recent Changes

Not applicable in the usual sense — the affected files are version-pinned plugin assets in the
read-only cache (`…/accelerator/1.22.0-pre.11/…`), not tracked in this repository, so per-file
`git log` history is not meaningful here. The relevant "change" is a standing authoring
convention rather than a regression in a specific commit.

## Open Questions

- Does Claude Code's matcher allow `*` to span `/` path separators? This affects whether a
  single user-settings glob like `Bash(bash /…/cache/*/scripts/artifact-*)` (Option C) reliably
  covers the versioned cache path, or whether a more permissive rule is needed.
- Is the first-call-only `allowed-tools` enforcement quirk (Hypothesis 3 / upstream issues)
  present in the reporter's Claude Code version? If so, even a correct bare-path setup could
  intermittently prompt, and Option B/C become more valuable as backstops.
