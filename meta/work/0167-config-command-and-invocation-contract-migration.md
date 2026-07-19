---
type: work-item
id: "0167"
title: "Built-in config Command and Invocation-Contract Migration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0164", "work-item:0166", "work-item:0178", "work-item:0179"]
blocks: ["work-item:0169", "work-item:0173", "work-item:0174"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0106", "work-item:0107", "work-item:0180"]
tags: [rust, config, skills, invocation-contract, allowed-tools, cli, migration]
last_updated: "2026-07-19T21:06:21+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-188"
---

# 0167: Built-in config Command and Invocation-Contract Migration

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Wire the full `accelerator config` command into the launcher over the shared
`config`/`corpus` crates — including the net-new `config set` write path — then
cut the invocation contract over: every **config-cluster call site** in a SKILL.md
moves from a bare script path to an `accelerator …` call, with its `allowed-tools`
rules rewritten in lockstep, behind the bootstrap path 0164 provides. Call sites
to non-config script families (`artifact-*` and the rest) stay on bare paths until
0173.

Also in scope, all of it load-bearing rather than incidental: migrating the config
hooks (SessionStart summary, `config-detect`) and defining the SessionStart
envelope 0169 inherits; replacing the removal set's shell test suites with Rust
tests — by **repointing** the existing suites at the binary as a parity gate and
inventorying only the subset that cannot repoint; extracting `atomic_write` into a
new `store` crate and consolidating any duplicate implementations; a committed
permission-coverage script and a `configure` round-trip harness; and recorded
updates to 0106, 0107, 0166, 0169, and the 0169–0174 dependency edges. This is the
highest-blast-radius story in the epic.

Terminology: **removal set** is the precise term for the scripts this story
deletes and whose call sites it rewrites — it is defined in the Acceptance
Criteria and committed as a file list. "Config cluster" appears in background
prose for the loose bash family (which includes `config-common.sh` and
`config-read-browser-executor.sh`, both of which survive); where a boundary
carries weight, the text says *removal set*.

## Context

ADR-0047 makes the CLI the native config reader and names `config get/set`;
ADR-0045 names the `configure` skill as the first proof of the skills-vs-CLI
division. The bash config cluster is the most-invoked code at skill-load time, and
every skill addresses it by bare path matched against `allowed-tools` prefix globs
(0106/0107). Moving to one `accelerator` command requires changing every call site
and glob together. Mirrors luminosity 0011 (configuration feature parity), but here
parity is with our own shell library plus the contract rewrite.

The surface is large but strikingly uniform, which is the main de-risking factor:
247 `!`-preprocessor call sites across 46 SKILL.md files, every one of the shape
`` !`${CLAUDE_PLUGIN_ROOT}/scripts/config-<name>.sh [one-arg]` `` — no wrappers, no
pipes, no quoting variants — and exactly **one** `allowed-tools` glob pattern
(`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`) across 35 frontmatter blocks. This
is also the first production exercise of the 0164 bootstrap path: `bin/accelerator`
is built and tested but referenced by zero skills and zero hooks today.

**Q2 is resolved (2026-07-19), and benignly.** Re-measured at revision
`b290d5d9`: 247 `!` call sites across 46 SKILL.md files; 35 files declaring
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` (34 clean plus one carrying a
trailing space). The eleven-file gap is **not** missing declarations — every one
invokes config scripts under a *broader* rule:

- **1 file** — `skills/vcs/commit/SKILL.md:7` declares
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)`, broad because it also calls
  `vcs-status.sh` and `vcs-log.sh`.
- **10 files** — the `disable-model-invocation: true` integration *write* skills,
  each declaring bare `- Bash`, each with the identical three-call-site shape.

**Nothing is broken today, and no file needs a rule added for the bash surface.**
Exactly one file can silently break under the rewrite: `skills/vcs/commit`, whose
`scripts/*` rule does **not** cover `bin/accelerator`. It gains a rule rather
than having one rewritten. `skills/config/configure/SKILL.md` also gains one — it
has no `allowed-tools` key at all today.

The `!`-scoped count is **not** the whole migration surface, which the earlier
text implied. Beyond the 247 there are **14 non-`!` call sites** across five
files — nine fenced code blocks in `configure/SKILL.md` (the only flagged and
multi-arg invocations anywhere), four prose "run this script" instructions, and
`skills/config/init/SKILL.md:45`. `create-jira-issue/SKILL.md:114` *sources*
`config-common.sh` and is out of scope. Grep A is the criterion that covers them.

## Requirements

- Implement the built-in `config` subcommand family (compiled into the launcher, no
  sub-binary fetch) reaching parity with the bash surface: `get`/`set`, `path`,
  `paths`, `context`, `agents`, `agent`, `template`, `templates
  list|show|eject|diff|reset` (ADR-0021, 0/1/2 exit codes), `doc-type-paths`,
  `work`, `review`, `dump`, `summary`, `skill-context`/`skill-instructions`
  (ADR-0020), `init`. `config set` is net-new.
- Keep the command interface consistent with luminosity where one exists: noun-verb
  grouping (`config get`, `config set`), positional args for the subject and named
  flags for modifiers, `--level team|personal` as a clap `ValueEnum`, and doc
  comments treated as contract (help text is asserted in tests). Where luminosity
  has no precedent — the `templates` group is unimplemented there (its 0019 is
  draft) — we set the precedent.
- Adopt luminosity's **command-level split** between machine-plain and
  prose-for-injection output rather than a `--format` switch: scalar reads emit one
  bare value plus a single newline, while the injection blocks (`## Agent Names`,
  `## Project Context`, `## Review Configuration`) are their own byte-exact
  commands. Preserve the meaningful exit codes (notably exit 2 = "not customised"
  across the three template-mutation paths, ADR-0021) — we diverge from
  luminosity's uniform exit 1 deliberately, because ADR-0021 is accepted and exit
  2 carries meaning our callers act on.
- **Consolidate `atomic_write` into a new `store` crate.** Sweep `cli/` for
  temp-file-plus-rename implementations, extract them into `store` behind one
  permitted-root-aware primitive, and repoint every caller. At minimum that is
  `config-adapters`' shipped implementation; if 0180 has landed by then, its
  `corpus-adapters` implementation too. Commit a check that fails on a
  reintroduced duplicate, so the sweep is a durable guarantee rather than a
  one-time tidy-up. See Dependencies for why this story owns it.
- **Parity strategy: repoint first, inventory the remainder.** Per the source
  research's cross-cutting Q7 strategy, the existing suites are *repointed* at the
  compiled binary as a black-box parity gate during cutover, then retired in the
  change that deletes the scripts. This is viable because `test-config.sh` binds
  each script path to a variable once (`READ_VALUE="$SCRIPT_DIR/config-read-value.sh"`,
  ~20 such bindings) and invokes them uniformly as `bash "$VAR" args`, so
  redirection is mechanical — either by redefining the bindings or by temporary
  shims that `exec accelerator config …` and are deleted with the suite.
  **A subset does not repoint**, and that subset alone needs a behaviour
  inventory:
  - **Call-site greps** (`test-config.sh` ~1096, 3013-3025, 4986-4990) count
    `config-read-*.sh` occurrences *inside SKILL.md files*. They assert the
    invocation contract this story rewrites, so they break by design and have no
    CLI analogue. Rewrite them against the new invocation shape or drop them with
    a recorded reason — do not silently delete.
  - **`config-defaults.sh` assertions** (~2441, 2530) treat the defaults as a
    shell file, which the Rust implementation will not have.
  - **Any removal-set script with no covering suite** — inventory it by reading
    the script. This is where silent loss is likeliest.
  Commit the inventory under `meta/`. It is the parity denominator for the
  remainder only; the repointed assertions are their own gate.
- Audit every config suite that survives the supersessions and record, per suite,
  whether it exercises any script in the removal set. Any that does is either
  inventoried and ported, or repointed — it cannot be left to break. Enumerate
  the suites rather than assuming a count: `test-init.sh` is wired in mid-story
  and retired later, so the registered population changes as the work proceeds.
- Provide splice-safe degradation for the `!`-preprocessor call sites. A non-zero
  exit from a spliced command discards the whole prompt, so injection commands must
  offer a fail-safe mode that renders errors as `## … Unavailable` notices on
  stdout and degrades per-source (an unreadable skill file still leaves the project
  block standing), with diagnostics on stderr only. This must preserve the existing
  bash posture: fail-open on config content, with the three deliberate fail-closed
  exceptions retained — frontmatter writeback primitives, the `work.integration`
  enum, and doc-type path safety.
- Wire skills and hooks onto the **bootstrap path** — the stable
  `${CLAUDE_PLUGIN_ROOT}` entrypoint that 0164 delivers and that this story is
  the first consumer of. (0169 calls this same artefact "the wrapper"; it is one
  object, and this story fixes its name to "bootstrap path".) Rewrite every
  **config-cluster call site** — each SKILL.md `!`-preprocessor invocation of a
  script in the removal set — from its bare script path to `accelerator …`;
  update the `allowed-tools` rules so they cover the new bootstrap path; update
  the 0106 bare-path contract. Skills may migrate
  incrementally — across separate phases, commits, or PRs, with bash and
  `accelerator` call sites coexisting behind an `allowed-tools` set covering both —
  but every config-cluster call site must be migrated for this story to be done.
- Re-home `config-read-browser-executor.sh`'s permission coverage. It is out of
  scope for the `accelerator config` surface (see Technical Notes) but rides the
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` glob this story retires, so it
  must gain its own narrow rule **in the same commit that removes the `config-*`
  glob from a given frontmatter block** — not merely somewhere in the story, since
  the migration is explicitly permitted to span commits and PRs.
- Move the SessionStart config summary to `accelerator config summary` and migrate
  the `config-detect` hook. Of the four registrations in `hooks/hooks.json`, this
  story owns exactly one: `config-detect`. `vcs-detect` and `vcs-guard` migrate in
  0169; **`migrate-discoverability` is 0172's** (the migration engine) — naming it
  here so it does not survive as unowned bash residue in a file this story
  otherwise edits. A temporarily mixed state is expected and acceptable.
- Emit the **SessionStart hook envelope** from the CLI via `--format=hook`, per
  the source research's resolved Q4. Scope is deliberately narrow: this story
  owns the SessionStart `additionalContext` envelope only. There is no single
  envelope spanning all hooks — `vcs-guard.sh` (PreToolUse) emits a completely
  different `{decision, reason}` shape — so **PreToolUse's envelope is 0169's to
  define**, and this story neither sets nor constrains it.
  The SessionStart envelope this story must reproduce, taken from
  `hooks/config-detect.sh` as it stands:

  ```json
  {"hookSpecificOutput": {"hookEventName": "SessionStart",
                          "additionalContext": "<summary text>"}}
  ```

  Three output states, all load-bearing:
  1. **Summary present** → the envelope above on stdout, exit 0.
  2. **Summary empty** → **emit nothing at all** — not `{}`, not an envelope with
     an empty string. Today's hook only prints when the summary is non-empty;
     emitting an empty envelope would inject a blank context block into every
     session.
  3. **Summary unavailable** (the read failed) → emit nothing, exit 0, diagnostic
     on stderr. This is the fail-open posture, consistent with the splice-safety
     requirement.

  The current jq-missing branch (`{"systemMessage": "…"}`) does **not** carry
  over: it exists only because the bash hook shells out to `jq`, and the Rust
  implementation serialises with `serde_json`. Removing it is a deliberate
  simplification, not an oversight.
- Invoke the hook and the skill-injection caller through the **same command**:
  `accelerator config summary` plain, `accelerator config summary --format=hook`
  wrapped. That is what makes the research's "one domain operation serves both
  callers" claim true. **Divergence from the research, recorded deliberately**:
  research Q4 names the command `accelerator config detect`, matching the
  `vcs detect` / `migrate discoverability` hook naming. This story uses `summary`
  instead, because `config-detect.sh` today is a 25-line wrapper that does nothing
  but call `config-summary.sh` — they are one operation, and `summary` is already
  in this story's enumerated subcommand surface while `detect` is not. If the epic
  prefers `detect`-naming symmetry across the three SessionStart hooks, that is a
  rename to settle with 0169 before either story ships, not two commands.
- Record the SessionStart envelope decision and the bootstrap-path naming on 0169,
  so the VCS hooks inherit a written contract rather than an observed one.
- Replace the removal set's shell test suites with Rust tests, via the
  repoint-then-retire sequence above: repointed suites act as the parity gate
  while the Rust tests are written, and are retired in the change that deletes
  the scripts they cover. Mirror luminosity's black-box CLI harness: spawn the
  real compiled binary, assert on exact stdout/stderr bytes where it matters, and
  build throwaway fixture workspaces containing a `.git` marker so root discovery
  is bounded inside the fixture.
- **Fail-safe is the default** for the injection commands, not an opt-in flag. A
  missed flag at a rewritten call site would discard the whole prompt in
  production while every test still passed, so the safe posture must not depend on
  remembering it. Provide an opt-*out* for debugging if useful.
- Handle `skills/config/init/scripts/test-init.sh` as **characterise-then-retire**,
  in that order, since this story also moves `init.sh` to `accelerator config init`
  and a suite covering a deleted script cannot pass. First wire it into
  `run_shell_suites` — it is executable but sits under `skills/config/init`, which
  is not one of the eight subtrees the task discovers, so it has never run in CI
  and we do not currently know that it passes. Fix whatever that surfaces. Then
  add its rows to the behaviour inventory, port them to Rust, and retire the suite
  in the same change that removes `init.sh`. The wiring is not busywork: it is the
  only way to learn whether the behaviour we are about to port is the behaviour
  the script actually has.
- Prove the round trip end-to-end on the `configure` skill first (ADR-0045).

## Acceptance Criteria

Grouped by theme. Every criterion states a procedure that can definitively fail;
those that run against a workspace name the fixture they use, defined in Technical
Notes. (Criteria in the Parity, Suite lifecycle, Invocation contract, Cross-item
records and Removal groups act on the repository rather than a fixture, so they
name none.)

### Command surface and output contract

- [ ] A checked-in **surface table** has one row per subcommand in the
      Requirements enumeration, with columns: subcommand, **output class**,
      **fixture**, and **expected exit code**. It is the parametrised test's input,
      so an omitted or stubbed subcommand fails. The classes and their contracts:
      - **scalar** (`get`, `path`, `agent`, `template`) — stdout is exactly the
        value plus a single `\n`; stderr empty.
      - **block** (`paths`, `context`, `agents`, `work`, `review`, `dump`,
        `summary`, `skill-context`, `skill-instructions`, `doc-type-paths`,
        `templates list|show`) — stdout matches a committed golden byte-for-byte.
      - **customisation-state** (`templates eject|diff|reset`) — the ADR-0021 exit
        codes below govern; stdout matches a committed golden per exit state.
      - **mutation** (`set`, `init`) — exit code plus a post-state assertion on
        the file touched.
      A subcommand's class may be corrected only by amending **this criterion**
      alongside the table, so the specification cannot be edited to match whatever
      was built.
- [ ] Exit codes, ADR-0021 (**customisation-state** class). ADR-0021:80 defines
      exit 2 as "destructive action requires confirmation". The three commands
      fire it on **opposite** customisation states, so no single fixture serves
      all three:

      | Command | Exits 2 when | Fixture | Exits 0 against |
      |---|---|---|---|
      | `templates eject` (no `--force`) | the override **already exists** | **already-customised** | **not-customised** |
      | `templates diff` | there is **no** override | **not-customised** | **already-customised** |
      | `templates reset` | there is **no** override | **not-customised** | **already-customised** |

      Against the **error** fixture all three exit **1**. A uniform-exit-1
      implementation fails this criterion. For `eject --all`, any error wins
      (exit 1) over any exists (exit 2), per `config-eject-template.sh:118-137`.
- [ ] **Usage errors exit 1, so exit 2 keeps one meaning.** clap 4 exits **2** on
      a usage error (unknown flag, bad `--level`) and the launcher currently
      delegates to `error.exit()` (`cli/launcher/src/main.rs:106`). The bash exits
      **1**, so without interception a mistyped template name would be
      indistinguishable from "confirmation required". Asserted by invoking a
      template subcommand with an unknown flag and observing exit 1.
- [ ] The three injection commands — `config agents` → `## Agent Names`,
      `config context` → `## Project Context`, `config review` →
      `## Review Configuration` — each match a committed golden byte-for-byte
      against the baseline fixture. Naming the subcommands, not only the headings,
      fixes the set the error-posture criteria below key off.
- [ ] `--help` on `config` and on every subcommand in the surface table matches a
      committed snapshot (doc comments are contract).

### Error posture

- [ ] Fail-safe is the **default** for the three injection commands: no flag is
      required at a call site to obtain it. Asserted by invoking each with no
      flags against the **unreadable-config** fixture and observing the fail-open
      behaviour below.
- [ ] **Fail-open**: given a config error **other than the three fail-closed
      triggers below**, an injection command exits 0 and stdout equals exactly
      `## Agent Names Unavailable\n` (or `## Project Context Unavailable\n` /
      `## Review Configuration Unavailable\n`), with the diagnostic on stderr and
      stdout carrying nothing else. The exclusion matters: without it, an invalid
      `work.integration` value met while rendering `## Project Context` would
      satisfy the antecedent of both this criterion and the fail-closed one, which
      demand opposite exit codes.
- [ ] **Per-source degradation**, asserted across invocations rather than within
      one, because the three blocks are separate commands and therefore separate
      processes: given one broken source (an unreadable skill file), the affected
      command emits its unavailable notice while the other two render their blocks
      normally.
- [ ] **Fail-closed**, the three deliberate exceptions, each with its own named
      fixture and exercising command: given the **writeback-failure** fixture,
      `config set` exits non-zero; given the **bad-integration-enum** fixture,
      `config work` exits non-zero; given the **doc-type-escape** fixture,
      `config doc-type-paths` exits non-zero. In all three, stdout is empty and the
      diagnostic is on stderr.

### Parity

- [ ] **Repointed suites pass against the binary.** `test-config.sh` and
      `test-config-read-doc-type-paths.sh` are redirected at the compiled
      `accelerator` (by rebinding their ~20 script-path variables or by temporary
      shims) and pass, before any script they cover is deleted. This is the parity
      gate for the bulk of the 337 assertions; the inventory below covers only what
      cannot be repointed.
- [ ] **The non-repointable remainder is inventoried and dispositioned**, each
      with a recorded outcome — ported, rewritten against the new invocation shape,
      or dropped with a reason. The remainder has **four** members, and the two
      gates are exhaustive over every superseded assertion between them:
      1. the call-site greps (`test-config.sh` ~1096, 3013-3025, 4986-4990);
      2. the `config-defaults.sh` file assertions (~2441, 2530);
      3. **all of `test-init.sh`** — it is not repointed (the suite has never run,
         so there is no trustworthy baseline to repoint), and `init.sh` has a
         covering suite so it falls outside member 4. Naming it explicitly closes
         the gap where the one never-run suite belonged to neither gate;
      4. every removal-set script with **no** covering suite.
      No silent deletions.
- [ ] The inventory is mechanically checkable: rows are keyed by `<file>:<line>`
      of the assertion or by script path, the mapping to named Rust tests is
      checked in, and a committed script asserts no duplicates and no gaps against
      a fresh extraction of the four remainder members. Its cardinality is
      recorded and reconciled against that extraction — **not** against the 337
      figure, which counts `test-config.sh`'s assertions and bears no relation to
      the remainder's size.
- [ ] **Depth floor for members 3 and 4**, which have no suite to bound them: for
      each such script, every branch of its top-level control flow and every
      distinct exit code it can produce is a separate inventory row mapped to a
      named Rust test. Without this, a single hand-wavy row per script satisfies
      the criteria above and the script becomes deletable — the exact silent-loss
      path the Assumptions call the likeliest.
- [ ] No script or suite is deleted before the assertions covering it either pass
      repointed or appear in the inventory mapped to a passing Rust test.

### Suite lifecycle

- [ ] **`test-init.sh` characterisation happened**: it is wired into
      `run_shell_suites` and observed **green in CI at a recorded commit** before
      any of its rows are ported. Failures surfaced by that first run are recorded
      with their resolutions, and its retirement commit references the recorded
      green run. Without this, deleting it alongside `init.sh` would satisfy every
      other criterion while skipping the characterisation that justifies the
      sequence.
- [ ] **Surviving-suite audit**, pinned to a revision: the checked-in audit table
      records the commit at which `run_shell_suites` discovery was run, every
      discovered suite appears as a row classified (a) exercises no removal-set
      script, (b) inventoried and ported, or (c) repointed, and the equality
      between table rows and discovery is reproducible at that revision. A
      **second, final-state discovery run** is recorded and every difference from
      the audit table is attributed to a named deletion or addition — the
      population changes mid-story, so a single unpinned count cannot hold.
      The audit **must** include `scripts/test-design.sh`, which is named nowhere
      else in this story but asserts SKILL.md invocation shape and the
      browser-executor's existence at `:42`, `:157-161`, `:427`, `:444-446`,
      `:471-472` — the same class as the flagged `test-config.sh` regions, and it
      breaks by design at the cutover.
      It must also record the two **Rust** tests pinning the shell surface, which
      break at deletion rather than at cutover:
      `cli/config-adapters/tests/parity.rs:42-43,113-121` (asserts
      `config-read-value.sh` is a file, then shells out to it) and
      `cli/corpus-adapters/tests/doc_type_single_source.rs:189-220` (sources
      `config-defaults.sh`); plus the two suites writing `exec` stubs that
      hard-code the resolver path
      (`scripts/test-validate-corpus-frontmatter.sh:412`,
      `skills/config/migrate/scripts/test-migrate-0007.sh:2208`).
- [ ] **At the final state** (after the deletion change — not at any intermediate
      commit, since `test-init.sh` is deliberately wired *in* mid-story), the set
      `run_shell_suites` discovers contains **none** of the three superseded
      suites (`test-config.sh`, `test-config-read-doc-type-paths.sh`,
      `test-init.sh`). `_EXPECTED_CONFIG_SUITES` is then set to equal what
      discovery finds — necessary bookkeeping, but tautological once edited, so the
      absence assertion is what carries weight. Note it is an **at-least floor**
      (`tasks/test/integration.py:85` compares with `<`), not an equality, so
      retiring suites requires *lowering* it or the build fails; the sibling
      floors (`_EXPECTED_MIGRATE_SUITES`, `_EXPECTED_WORK_SUITES`,
      `_EXPECTED_INTEGRATIONS_SUITES`) work the same way and are untouched.

### Invocation contract

- [ ] **Grep A — removal-set coverage.** A literal command over the same corpus as
      Grep B — `--include=SKILL.md skills/`, plus `hooks/` — whose pattern spans
      every path in the committed removal-set file list (which excludes
      `config-read-browser-executor.sh`). The corpus must be fixed in the criterion,
      not chosen at verification time: run tree-wide the pattern would also match
      this work item, the research document, and `tasks/`, so "exactly 0" would be
      unreachable and a scope picked afterwards could be narrowed until it passed.
      Post-migration it returns **exactly 0**.
      Its pre-migration run is recorded as the known-positive floor — a mistyped
      pattern also returns zero, so the pre-run must demonstrably find call sites.
      This is the denominator for "every call site": it is the only check covering
      `skills/config/init/scripts/init.sh` and the per-skill readers, whose paths
      contain no `config-` segment.
- [ ] **Grep B — residual check.** The literal `grep -rn 'scripts/config-'
      --include=SKILL.md skills/`, run pre-migration to record both its total and
      its `config-read-browser-executor.sh` subset. Post-migration it returns
      **exactly that subset count**, and every remaining hit is one of them.
      Measured at revision `b290d5d9`: total **297**, browser-executor subset
      **1**, so post-migration it must return **exactly 1**. (The 297 includes the
      35 `allowed-tools` declaration lines, which the rewrite also removes — the
      pre-migration figure is a floor to re-measure at cutover, not a constant.)
      (Two greps because one cannot do both jobs: Grep A's pattern is built from
      removal-set paths and can never match the browser-executor, whose surviving
      call sites are exactly what Grep B counts.)
- [ ] **Permission coverage.** A committed script extracts, for every SKILL.md,
      **every `!`-preprocessor invocation — bare-path and `accelerator` alike** —
      and asserts each is covered by at least one `Bash(...)` rule in that file's
      frontmatter under the matcher semantics resolved in Q1. It exits non-zero on
      any uncovered invocation. Extraction must include bare paths: the
      browser-executor is invoked that way, and an `accelerator`-only extractor
      would never examine the one call the re-homing requirement exists to protect.
- [ ] **Same-commit re-homing.** The coverage script is replayed against **each
      commit** in the migration range and exits zero at every one; the replay
      output is recorded. Final-tree checks cannot distinguish a rule added in the
      right commit from one added three commits later, so this replay — over an
      extractor that sees bare paths — is what verifies the requirement.
- [ ] Q1's answer, and the Claude Code version it was empirically verified
      against, are recorded in Assumptions before the first call site is rewritten.
- [x] Q2's re-measured counts and the explanation for the 46-vs-35 gap are
      recorded in Context before the first call site is rewritten, and any file
      found to invoke a config script without declaring the permission gains a rule
      rather than a rewrite. **Done 2026-07-19** — see Context. No file lacks a
      declaration; `skills/vcs/commit` and `skills/config/configure` gain rules for
      the new bootstrap path.

### End-to-end proof (ADR-0045)

- [ ] A committed harness drives the `configure` skill against the baseline
      fixture: it extracts every `` !`…` `` command from `configure/SKILL.md`,
      executes each, and concatenates the results in document order; that
      concatenation matches a committed golden capture. **Every extracted command
      that invokes a removal-set script routes through `accelerator`**; any
      remaining bare-path invocations are enumerated in the change and each shown
      to belong to a non-config family deferred to 0173 — so the criterion does not
      require doing 0173's work.
- [ ] The same harness run exercises the write path end-to-end: a `config set`
      through `configure`, then a re-read returning the written value. This is the
      ADR-0045 round-trip proof, and it is the only criterion tying the read and
      net-new write paths together through a real skill rather than through unit
      fixtures. A manual in-session check is additionally useful but is not the
      criterion — the harness is, so it re-runs as a regression guard.

### Hooks

- [ ] Before `config-summary.sh` is deleted, its output against the baseline
      fixture is captured and committed as a golden, and `accelerator config
      summary` against that same fixture matches it byte-for-byte.
- [ ] `accelerator config summary --format=hook` against the baseline fixture
      emits exactly
      `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":…}}`
      with the plain command's output as the `additionalContext` value; against the
      **empty-summary** fixture it emits **nothing** on stdout and exits 0; against
      the **unreadable-config** fixture it emits nothing on stdout, exits 0, and
      writes a diagnostic to stderr.
- [ ] **Live hook equivalence**, comparing like with like: in a named scratch
      repository, capture (a) `accelerator config summary --format=hook` invoked
      directly, and (b) the `additionalContext` value delivered by a real
      SessionStart in that same repository state. Parse the `additionalContext`
      field out of (a) and assert it is byte-identical to (b). Both captures are
      attached to the change. (Comparing the whole envelope from (a) against the
      field from (b) would compare a JSON object to one of its string values and
      could never pass.)
- [ ] The `config-detect` registration in `hooks/hooks.json` invokes the bootstrap
      path with `config summary --format=hook`. `vcs-detect`, `vcs-guard`, and
      `migrate-discoverability` remaining on bash is expected.

### `config set` and the `store` crate

- [ ] Given a `config set` on `.accelerator/config.md` or `config.local.md`, the
      value re-reads identically; all content outside the edited frontmatter key —
      including the surrounding Markdown body prose — is byte-identical to the
      pre-write file; and `config.local.md` is gitignored.
- [ ] Given each of the three **malformed** fixtures (unterminated frontmatter,
      invalid YAML, config-dir symlink escape), `config set` refuses the write and
      leaves the existing file untouched.
- [ ] Given a path whose symlink resolves outside the permitted root,
      `atomic_write` refuses on **both** read and write rather than following it.
- [ ] The `store` crate's write path is exercised through an injected filesystem
      port whose recorded call sequence contains no `open`/`create` on the target
      and exactly one `rename` onto it. Structural, not raced — observing
      concurrent iterations proves nothing when a non-atomic implementation can win
      every race. Separately, after both a successful and a failed write, no temp
      artefacts remain.
- [ ] **No temp-file-plus-rename implementation exists outside the `store`
      crate**, asserted by a committed grep or pup rule, save two allowlisted
      exceptions each carrying its reason inline. The check is shown to flag both
      real duplicates present in the pre-consolidation tree —
      `cli/config-adapters/src/store.rs:58-80` and
      `cli/corpus-adapters/src/store.rs:48-85`, **two, since 0180 has landed** —
      and those findings are recorded. Phrased as "none outside `store`" because
      that is what the check proves; it does not establish that exactly one exists
      inside.
      **The two exceptions are not duplicates and must not be flagged**, or the
      check fails forever:
      - `cli/launcher/src/launch/outbound/resolve/cache.rs:112-127` — a cache
        *publication* primitive: 0600 write, conditional `chmod +x`, and a paired
        `.minisig` written alongside. Its permission semantics have no analogue in
        a config or corpus write.
      - `cli/corpus-adapters/src/lock.rs:106-117` — renames a **directory** as a
        stale-lock claim. Not a write at all; a naive `fs::rename` grep hits it.
      A raw `fs::rename` grep is therefore insufficient on its own — the check
      needs the allowlist, or a shape-aware pattern that matches temp-write-then-
      rename-a-file specifically.
- [ ] `config set` calls `store`'s `atomic_write` and contains no temp-file or
      `fs::rename` logic of its own.

### Cross-item records

- [ ] 0106's canonical bare-path directive — the sentence its Drafting Notes
      designate authoritative, held in **0106's plan** — gains an
      `accelerator`-shaped variant for config-cluster invocations, with 0106's
      work-item blockquote updated to match so the two do not diverge. The existing
      bare-path directive is **retained unchanged** for `artifact-*` and other
      non-config families, which stay on bare paths until 0173.
- [ ] The migrate-then-build disposition for 0107 is recorded on 0107, with the
      invocation shape its future matcher must cover.
- [ ] The SessionStart envelope contract and the bootstrap-path naming are recorded
      on 0169, which also notes that PreToolUse's envelope is 0169's own to define.
- [ ] 0166's resolved store-crate decision carries a recorded amendment naming
      `store` as the split that fired, referencing this story.
- [x] Each of **0169, 0173 and 0174** — the three items in this story's
      frontmatter `blocks` — carries `blocked_by: "work-item:0167"`, so those edges
      are readable from either end. **Done 2026-07-19**: 0169 and 0174 already had
      it; 0173 had no `blocked_by` field and gained one. 0170-0172 are deliberately
      excluded: they are downstream of 0173's pattern rather than of this story's
      output, are recorded in prose only, and adding a one-sided `blocked_by` for
      them would create the asymmetry this criterion exists to remove.

### Removal and green build

- [ ] The **removal set** is committed as an explicit file list alongside the
      inventory — not a category description. It comprises: the `config-read-*`
      family **except** `config-read-browser-executor.sh`; `config-dump.sh`;
      `config-summary.sh`; the per-skill readers; the template-management scripts;
      and `skills/config/init/scripts/init.sh`. `scripts/config-common.sh` is
      **not** on it — its retirement is 0174.
- [ ] `mise run check` and the bare `mise run` pass with every file on the removal
      set deleted.
- [ ] The `config-common.sh` sourcer count is re-measured post-migration and
      recorded alongside the pre-migration figure (44 today). The surviving count
      is what justifies keeping the library until 0174; if it reaches zero, say so
      rather than deferring on a stale number.

### Performance

- [ ] Given a warm binary cache and the baseline fixture, the p95 of `accelerator
      config path <key>` over 100 runs is **no greater than** the p95 of the
      equivalent bash invocation captured in the same run on the same host. Both
      numbers are recorded. Self-relative by construction, so no reference machine
      need be specified — and a faster-than-bash result passes.
- [ ] The `config` subcommand's crate graph contains no HTTP or fetch dependency,
      asserted by a committed dependency check. A static property; runtime
      "unreachability" is not observable with confidence.

## Open Questions

- **(Q1, blocks the rewrite)** Does the `allowed-tools` prefix matcher's `*` span
  `/`? Still unanswered from 0107, and it decides whether a single
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` glob is viable or narrower
  per-subcommand globs are required. **Resolution path**: an empirical probe
  against the minimum supported Claude Code (v2.1.144) before the first call site
  is rewritten; record the verified version alongside the answer, since this is
  undocumented vendor behaviour that may change.
  **Counter-evidence worth probing against**: the tree carries
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/*)`
  **alongside** `.../inventory-design/scripts/*`. That nested rule would be
  redundant if `*` spanned `/`, which cuts against the source research's reading
  of the same taxonomy as evidence that it does. Neither reading is assumed; the
  probe settles it.

**Q1 is the only outstanding item.** Q2 was resolved on 2026-07-19 by
re-measurement — see Context.

**Closed decisions** (recorded here rather than left in the list, so Open
Questions stays a list of things that actually block work):

- **No `--format` switch for output shape.** It would be net-new invention, not
  parity: no config script accepts one (flag parsing across all 21 **config
  script entrypoints** is limited to `--force`/`--dry-run`/`--all` on
  `config-eject-template.sh` and `--confirm` on `config-reset-template.sh`), and
  luminosity has none. Both surfaces converge on the command-level split, which
  the Requirements adopt. Reopen only if a machine-readable rendering of the
  injection blocks turns out to be needed.
  **`--format=hook` is an accepted exception, and the collision is real rather
  than apparent** — it is literally the flag this decision rejects. The
  justification (the split governs how a value is *rendered*; `--format=hook`
  only wraps an already-rendered value in a transport envelope for a second
  caller) is genuine but thin enough that someone reading both would reasonably
  see a contradiction. Recorded so they don't have to guess. If a cleaner
  separation is wanted later, use a distinct flag name (`--envelope=hook`) rather
  than reopening the split.
- **`config set` preserves the surrounding Markdown body** — see Drafting Notes.
- **Exit-code taxonomy preserves ADR-0021's 0/1/2** — see Drafting Notes.

*(Two unrelated counts in this document both happen to read 21 at time of writing:
the **registered shell suites** in `_EXPECTED_CONFIG_SUITES`, and the **config
script entrypoints** referenced above. The coincidence is not a correspondence,
and the suite figure is a point-in-time reading — the suite-audit criterion
requires it to be re-measured rather than assumed.)*

## Dependencies

- Blocked by: 0166 (shared config/corpus crates) — concretely its children 0178
  (config/config-adapters, **done**) and 0179 (document/corpus crates, **done** as
of 2026-07-19 — the `document`, `corpus`, `corpus-adapters`, `vcs` and
`vcs-adapters` crates are all in the tree),
  which are what actually gate this work; and 0164 (the bootstrap + launcher,
  **done** — the entrypoint exists and is tested, this story is its first
  consumer). 0180 is **not** a blocker: this story consolidates `atomic_write`
  itself rather than waiting on 0180 to build it (see below), so 0180 sits in
  `relates_to`.
- **This story owns `atomic_write` consolidation — detection, extraction, and the
  `store` crate.** Deliberately *not* delegated to 0180. The current state,
  established by inspection: 0178 (**done**) already shipped an `atomic_write` as
  a private method on `FileConfigStore` (`cli/config-adapters/src/store.rs:58`) —
  temp dir under `config_dir()/tmp`, PID+counter-named temp file, write, rename,
  cleanup on both failure paths. **0180 has since landed** (2026-07-19; commits
  `338dcd37 → accc29a5 → 76753652 → 609bb999`), so the second implementation now
  exists in `cli/corpus-adapters/src/store.rs:48-85` — a free function taking
  `&[u8]` and returning `StoreError`, with a same-directory `NamedTempFile`, RAII
  cleanup and EXDEV classification, against config-adapters' method taking `&str`
  and returning `ConfigError`. The extraction reconciles all three axes.
  **The ordering-independence argument held**: there are two implementations to
  consolidate rather than one to extract. Either way this story converges on a
  single primitive, and
  0180 needs no amendment, no scope expansion, and no agreement negotiated before
  work starts. The earlier draft made 0180 the owner and required its consent as
  a precondition — unenforceable, since 0180 is a sibling that may proceed
  independently. **0180 is therefore not a blocker of this story** and has moved
  from `blocked_by` to `relates_to`.
  `store` is the crate name 0166 reserved: it rejected a standalone store crate,
  folded the capability into `corpus-adapters`, and left "a later split open if a
  second consumer needs it independently". `config set` is that second consumer,
  so the contingency has fired. Record the amendment on 0166 — but as a note, not
  a precondition; nothing here waits on it.
  **Scope is `atomic_write` only.** 0180's other two primitives — the mkdir-lock
  and canonical-order JSONL compose/remove — have corpus-only callers and are not
  duplicated, so they stay wherever 0180 puts them. `store` begins as a
  single-primitive crate; folding the rest in later stays open on the same
  second-consumer test 0166 applied.
  **Carried with it**: the symlink-escape refusal this story requires, generalised
  as a permitted-root parameter (refuse any component resolving outside the
  caller's root) so corpus gets the same guarantee rather than config-specific
  logic living in a shared crate. The shipped implementation has no such refusal —
  a real gap, not merely a relocation.
  **Knock-on work, none of it optional**: a new `cli/Cargo.toml` workspace member;
  a `pup.ron` rule for `store` (infrastructure, so it may import std and
  `kernel::Error` — and the existing domain-crate rules must keep
  `config`/`corpus`/`vcs` from importing it directly, since they may name only
  std, `kernel::Error`, and `crate`); and a check of whether `deny.toml` needs an
  entry (the primitive uses only `std` today, so probably not — confirm rather
  than assume). `cargo-pup`/`cargo-deny` policy is 0162's; coordinate there if the
  rules resist.
- Blocks: 0169 (VCS hooks reuse the bootstrap path + contract), 0173 (subdomain
  call-site rewrites follow the established pattern), 0174 (`config-common.sh`
  retirement builds on the reader removal here — 0174 is its sole owner; earlier
  drafts said "0173/0174", which left it able to fall between them). 0170, 0171,
  and 0172 also consume the invocation contract established here — 0169/0173/0174
  are called out as the nearest consumers, but a contract change during this
  story must be re-checked against all subdomain migrations 0169–0174. 0168
  (visualiser fold-in) is excluded deliberately: it consumes the launcher
  dispatch, not the SKILL.md invocation contract.
  The frontmatter `blocks` list holds 0169, 0173, and 0174 — the three items that
  consume the contract directly. 0170, 0171, and 0172 consume it too but are
  recorded in prose only, deliberately: they are downstream of 0173's pattern
  rather than of this story's output. **Settled and done (2026-07-19)**: every
  `blocks` edge is now bidirectional. 0169 and 0174 already carried
  `blocked_by: "work-item:0167"`; only 0173 lacked the field, and it has been
  added. (An earlier review finding asserted that *none* of 0169-0174 carried a
  reciprocal edge — that was wrong for two of the three, and the claim was
  repeated here before being checked.) The prose-only three stay prose-only.
- Relates to: 0106 (bare-path invocation — contract updated here), 0107 (lint
  guard — **not yet implemented**; this story defers building it and records the
  migrate-then-build disposition on 0107, so 0107 is *not* on this story's
  critical path).
- 0165 (distribution pipeline) is **status: done**, so its ship-gate is
  discharged in principle — but this story is the first production exercise of
  the fetched launcher, and an unfetchable or unverifiable launcher fails all
  migrated call sites at skill-load time. Reduce the gate to a verification step:
  confirm signed, checksum-verified artefacts exist for **every** supported
  platform before the first migrated call site reaches users. The migration and
  its tests need only a locally built binary, so this does not gate the work.
- External: Claude Code's `allowed-tools` prefix-matcher semantics (see Q1) and
  its `${CLAUDE_PLUGIN_ROOT}` expansion order are undocumented vendor behaviour
  this story depends on. Record the Claude Code version the behaviour was
  verified against.
- Parent: epic 0136.

## Assumptions

- The bootstrap path stays under `${CLAUDE_PLUGIN_ROOT}` so permission matches
  hold — resolved in the source research
  (`meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`),
  whose Q3 concerned the bootstrap path's location.
- `config` ships as a launcher **built-in**, not an external subcommand. An
  external would cost a second fetch-verify-cache round trip on top of the
  launcher's own, on a path that is invoked at skill-load time where latency is
  user-visible — `config-read-path.sh` alone has 66 call sites, and the bash
  cluster is already hand-tuned to a 20-30ms band.
- Parity is measured in two parts, per the repoint-first strategy: the **repointed
  suites** running green against the compiled binary are the gate for the bulk of
  the behaviour, and a **remainder inventory** covers only what cannot repoint
  (the call-site greps, the `config-defaults.sh` file assertions, all of
  `test-init.sh`, and removal-set scripts with no covering suite). Behaviour in
  that remainder which is not inventoried and mapped to a Rust test before
  deletion is lost irrecoverably — which is why the remainder carries a depth
  floor. A script with no covering suite remains the likeliest site of silent
  loss, and the repointing decision does not change that; it only removes the
  need to hand-inventory the assertions a suite already encodes.
- **RESERVED SLOT — TO BE FILLED before the rewrite starts (Q1).** The
  `allowed-tools` prefix matcher's `*` {does / does not} span `/`, verified
  empirically against Claude Code v{version}. This bullet asserts nothing until
  filled; it is a slot, not an assumption. It lives here rather than in Q1
  because the answer is a premise the rest of the plan rests on, and it is
  undocumented vendor behaviour that may change — so the verified version matters
  as much as the answer.
- `scripts/config-common.sh` survives this story for its non-config consumers. 44
  scripts source it today; an unknown number of those are themselves on the
  removal set, so the surviving count is lower and is re-measured and recorded per
  the removal-and-green-build criteria. The *surviving* count is what justifies
  keeping the library until 0174 — if it reaches zero, retirement moves earlier.

## Technical Notes

- Source bash surface: the `config-read-*` family, `config-dump.sh`,
  `config-summary.sh`, template-management scripts, per-skill readers, and
  `skills/config/init/scripts/init.sh`.
- `config-read-browser-executor.sh` is **out of scope**. It reads no config — it
  resolves a path under the plugin root and fail-closes if `run.sh` is not
  executable — and carries the `config-` prefix by naming accident, riding the
  `config-*` glob for that reason alone. Folding it into `accelerator config` would
  encode the accident into the new CLI surface. Its glob coverage must still be
  re-homed when the `config-*` pattern is retired. Note this diverges from the
  source research, which listed `accelerator config browser-executor` in the
  target surface. **0173 owns its eventual migration** (under whatever name is
  right for what it actually does) — recording an owner here so it does not
  survive the epic as unowned bash residue.
- `config set` follows luminosity's write discipline: write to a temp file,
  `fs::rename` onto the target, clean up on either failure, and run a
  symlink-escape refusal on every read *and* write so a component resolving
  outside the config dir is refused rather than followed. **This story consumes
  the `store` crate's `atomic_write` and writes no temp-and-rename logic of its
  own** — see Dependencies for the extraction.
- Note the existing implementation places its temp file in
  `config_dir()/tmp` — a *subdirectory* of the target's directory, not the same
  directory. `rename` is still atomic (same filesystem), but 0180's requirement
  says "same-directory temp file". The extracted primitive must settle on one
  placement; same-directory is the safer default, since a `tmp` subdirectory on a
  different mount would silently degrade `rename` to a copy.
- **Build-system terms** used throughout, defined once: `run_shell_suites` is the
  shell-suite discovery-and-run helper in the Python `tasks/` toolchain;
  `_EXPECTED_CONFIG_SUITES` is its expected-count guard for the config group,
  which fails the build when discovery finds a different number (the tripwire
  that stops a suite silently disappearing); and "the eight subtrees" are the
  fixed directory list `run_shell_suites` walks — `skills/config/init` not being
  among them is why `test-init.sh` has never run. See `tasks/README.md` and the
  test tasks under `tasks/`.
- **Fixture workspaces**, named once here and referenced by name from the
  criteria: a **baseline** fixture (team config present, no local overrides — the
  golden-comparison target for scalar reads, injection blocks, and the summary);
  a **not-customised** fixture (no ejected templates — the exit-2 state for
  `diff`/`reset` and the exit-0 state for `eject`); an **already-customised**
  fixture (an ejected template present — the exit-2 state for `eject` and the
  exit-0 state for `diff`/`reset`); an **error** fixture (for exit-1 paths); an
  **empty-summary** fixture;
  an **unreadable-config** fixture; and three **malformed** fixtures for
  `config set` (unterminated frontmatter, invalid YAML, config-dir symlink
  escape); and three **fail-closed trigger** fixtures — **writeback-failure**
  (a config file made unwritable), **bad-integration-enum** (`work.integration`
  set to an unrecognised value), and **doc-type-escape** (a doc-type path
  resolving outside the permitted root). Each contains a `.git` marker so root
  discovery is bounded inside it.
- The removal set's two category members must be enumerated as files before
  anything is deleted: **the per-skill readers** and **the template-management
  scripts**. Both are finite and gettable; commit the list rather than leaving
  the criterion to interpretation.
- On the grep exclusions: `config-common.sh` is *sourced* by scripts, not invoked
  from any SKILL.md body, so it produces no `scripts/config-` match and is not an
  exclusion in either grep. `config-read-browser-executor.sh` is handled
  differently by each: **Grep A** never matches it (its pattern is built from
  removal-set paths, and the browser-executor is not on that list), so it needs no
  exclusion there; **Grep B** matches it deliberately, because counting its
  surviving call sites is that grep's whole purpose. Describing it as "the only
  exclusion" was accurate only under the earlier single-grep phrasing.
- `scripts/test-config.sh` is 337 assertions across 6,289 lines (215 KB) — the
  largest test asset in the repo and the dominant migration cost.
- This story carries the largest behavioural-parity risk.

## Drafting Notes

- Treated as the Phase 4 story; deliberately bundles the `config` command with the
  invocation-contract rewrite because they must land together for skills to keep
  working. Kept as a single work item despite its size — a half-migrated invocation
  contract is the failure mode most worth avoiding — while allowing the
  implementation to split across phases, commits, and PRs.
- Dropped the `--format` requirement after checking both surfaces: neither the
  bash config cluster nor luminosity has a `--format` flag, and both already shape
  output per-command rather than per-flag. The original requirement appears to
  have been invented at extraction time rather than derived from the surface being
  migrated. Recorded as an open question rather than a settled decision.
- Excluded `config-read-browser-executor.sh` on the grounds that its `config-`
  prefix is a naming accident, not a statement about what it does.
- Narrowed the shell-removal criterion from "the config cluster's shell scripts" to
  the reader entrypoints after finding `config-common.sh` has 44 consumers beyond
  the config-reading surface.
- Reframed the 0107 relationship: the guard is a draft spec with no implementation,
  so it cannot be "updated" as originally written.

### Review pass 1 (2026-07-18)

- **Kept as one story** despite the reviewer's split proposal. The coexistence
  mode the Requirements permit is an *implementation* convenience — bash and
  `accelerator` call sites can overlap mid-flight — not a delivery boundary. A
  released state with the contract half-migrated is the failure mode this story
  exists to avoid, and shipping the command without the cutover would leave a
  second reader implementation live in the product with no forcing function to
  finish. The seam is real; it is deliberately not a release boundary.
- **Deferred 0107's guard** (migrate-then-build). The Summary previously claimed
  the guard was rewritten in lockstep, which no requirement supported; that claim
  is removed. Verification of the cutover rests on the grep denominator in the
  Acceptance Criteria — the same approach that verified 0106 — and 0107 stays off
  this story's critical path.
- **Settled the exit-code taxonomy** in favour of preserving ours. ADR-0021 is
  accepted and exit 2 = "not customised" is acted on by callers, so diverging
  from luminosity's uniform exit 1 is deliberate. The open question is closed and
  the requirement now states the divergence rather than leaving it contested.
- **Settled `config set` body preservation**: the surrounding Markdown prose
  survives byte-identically, following luminosity's `document::render`. Silent
  loss of user-authored prose in `.accelerator/config.md` is the worst plausible
  defect in a net-new write path, and the previous AC (value re-reads
  identically) could not detect it.
- **Kept `config set` in scope** rather than extracting it. It is the only write
  path in the config surface and the `configure` skill's round-trip proof needs
  it; splitting it out would leave that proof read-only.
- Converted the parity criterion from "every documented behaviour is covered" —
  which named no source of truth and reduced circularly to "port the suites" — to
  a committed behaviour inventory that acts as a countable denominator. This was
  the review's only critical finding.
- Added acceptance criteria for surfaces that had requirements but no definition
  of done: hook migration, the `configure`-first round trip, the byte-exact
  output contract, and the fail-closed exceptions (whose behaviour the previous
  splice-safety criterion contradicted by asserting exit 0 unconditionally).
- Reframed the runtime "no added permission prompts" criterion into inspectable
  static conditions, following the precedent 0106 set, and phrased glob coverage
  in terms of coverage rather than cardinality so either resolution of Q1
  satisfies it.
- Qualified "every SKILL.md call site" to the config cluster throughout; the
  unqualified phrasing read as the whole plugin and collided with 0173's scope.
- Fixed the frontmatter dependency graph (`blocked_by` was absent entirely;
  `blocks` omitted 0174, which declares itself blocked by this story), named
  0166's concrete gating children, and recorded the 0165 and Claude Code
  matcher couplings.

### Review pass 2 (2026-07-19)

Pass 1's edits resolved the critical parity finding but introduced two defects of
their own; this pass repairs those and closes the remaining verification gaps.

- **Widened the behaviour inventory to the whole removal set.** Pass 1 scoped it
  to the two named suites while the removal criterion deleted a much broader
  script family — template-management scripts, per-skill readers, `init.sh` — so
  the critical risk was narrowed rather than closed. Scripts with no covering
  suite must now be inventoried by reading the script. Added a granularity floor
  (every one of the 337 assertions attributed to exactly one row), because "one
  row per assertion group" was unbounded and a 20-row inventory satisfied it as
  well as a 300-row one. Added an audit of the nineteen surviving suites so none
  is left silently exercising a deleted script.
- **Resolved the `test-init.sh` / `init.sh` contradiction** as
  characterise-then-retire. Pass 1 required the suite to be discovered and passing
  while also deleting the script it exercises — the two could not both hold. The
  wiring is kept because the suite has never run in CI, so we do not know the
  behaviour we are about to port is the behaviour the script has; it is then
  inventoried, ported, and retired with `init.sh`.
- **Settled `atomic_write` ownership**: the primitive moves down into a shared
  lower-level crate that both config and corpus depend on, rather than
  config-adapters depending on corpus-adapters or the write discipline being
  implemented twice. ~~This expands 0180's scope and must be agreed there.~~
  **Superseded by pass 3** — this story owns the consolidation outright, so 0180
  needs no amendment and no negotiated consent. See "Review pass 3 —
  `atomic_write` settled" and the Dependencies section.
- **Made the suite counter self-checking** (`_EXPECTED_CONFIG_SUITES` equals what
  discovery actually finds) instead of deferring its own pass condition to the
  implementer, which is what "plus one if … confirm at implementation" did.
- **Named a procedure for the permission-coverage check.** With 0107 deferred and
  Q1 open, the story's highest-risk property had neither automation nor a manual
  procedure — and the grep proves only the *absence* of old paths, not that new
  invocations are covered. A throwaway coverage script is now required.
- Added criteria for three things the Requirements asserted but nothing verified:
  the enumerated subcommand surface, the ADR-0021 exit-code taxonomy (previously
  satisfiable by uniform exit 1, defeating the pass-1 decision), and the 20-30 ms
  latency band that justifies compiling `config` in as a built-in.
- Reframed the atomicity criterion from a race condition no test can conclusively
  fail into a structural assertion (target never opened for writing; single
  rename; 100 iterations leave no intermediate state or temp artefacts).
- Corrected pass-1 text: the `config-summary.sh` golden must be captured *before*
  the script is deleted; the `configure` round trip names its capture harness;
  the 0106 criterion no longer implies deleting the `artifact-*` directive that
  must survive until 0173; `0169's wrapper model` no longer contradicts the
  bootstrap-path naming; the garbled "to the config cluster" insertions are
  rewritten; and the Drafting Notes no longer cite AC numbers that do not exist.
- Corrected the dependency record: 0164 and 0165 are both **done** and were
  described in the wrong tense; 0178/0179/0180 are now in `blocked_by` and not
  only in prose; `config-common.sh` retirement has one owner (0174) rather than
  two; 0168 is documented as a deliberate exclusion from the consumer sweep; and
  `config-common.sh` is no longer listed as a grep exclusion, since it is sourced
  by scripts and never invoked from a SKILL.md body.
- Added the hook output-envelope requirement. The source research resolved it via
  `--format=hook`; this story's decision against `--format` concerned output
  *shape*, which is a different question, and conflating them would leave 0169
  matching an undocumented envelope.

### Review pass 3 — hook envelope narrowing (2026-07-19)

- **Narrowed the hook-envelope requirement from "define and own the envelope" to
  the SessionStart `additionalContext` envelope alone.** The earlier phrasing was
  unsatisfiable in principle: inspecting `hooks/` shows there is no single
  envelope. The three SessionStart hooks emit `hookSpecificOutput`,
  `vcs-guard.sh` (PreToolUse) emits an unrelated `{decision, reason}` shape, and
  `migrate-discoverability.sh` emits no JSON at all — plain text to stderr. A
  requirement to define one envelope spanning those could not be met.
- **Stated the envelope concretely** rather than instructing the reader to decide
  it. The prior draft said "state it here" and never did, while an acceptance
  criterion asserted conformance to "the hook envelope defined in Requirements" —
  a reference with no referent, and a criterion any implementation satisfied by
  construction. Four of five review lenses flagged it independently.
- **Named all three output states**, because the empty case is load-bearing:
  `config-detect.sh` prints only when the summary is non-empty, so an
  implementation emitting `{}` or an empty-string envelope would inject a blank
  context block into every session — a regression no golden comparison on the
  non-empty path would catch.
- **Dropped the jq-missing `{"systemMessage": …}` branch** deliberately. It exists
  only because the bash hook shells out to `jq`; `serde_json` removes the
  dependency and the branch with it.
- **Chose `config summary` over the research's `config detect`** and recorded the
  divergence. `config-detect.sh` is a 25-line wrapper around `config-summary.sh` —
  one operation, not two — and `summary` is already in the enumerated subcommand
  surface. The research's "one domain operation serves both callers" only holds
  if the skill and the hook call the *same* command, which the previous draft
  broke by pairing skill-side `summary` with research-side `detect`. If
  `detect`-naming symmetry across the three SessionStart hooks is wanted, that is
  a rename to settle with 0169, not a second command.
- **Recorded the `--format` collision honestly** in the closed `--format` decision
  (later moved out of Open Questions) rather than resting on the
  transport-vs-rendering distinction. `--format=hook` is literally the flag Q3
  rejects; the distinction is genuine but thin, and a reader meeting both without
  explanation would reasonably see a contradiction. `--envelope=hook` is noted as
  the escape hatch if a cleaner separation is wanted.
- Assigned `migrate-discoverability` to 0172 so all four `hooks.json`
  registrations have an owner, and named the fixture workspaces once in Technical
  Notes so the criteria stop referring to "the named fixture workspace" without
  an antecedent.

### Answering the scope objection (2026-07-19)

Raised in all three review passes and deferred twice, so recorded properly here.
The objection, at its sharpest: the single-story rationale ("a half-migrated
invocation contract is the failure mode most worth avoiding") justifies coupling
the `config` command to the call-site cutover, but it does *not* justify bundling
the behaviour-inventory-and-test-port programme, which the revisions have grown
into the dominant thread. That thread's forcing function is *script deletion*,
not contract integrity — and deletion is demonstrably schedulable separately,
since `config-common.sh`'s retirement is already deferred to 0174 on exactly that
reasoning. So the cutover is gated behind the slowest and riskiest work in the
epic.

**What's right about it.** The stated rationale genuinely doesn't cover the test
programme. Answering it required a different argument, not a restatement of the
first one, and the deferrals were avoidance.

**The answer: the inventory is upstream of the command, not downstream of the
cutover.** To implement `accelerator config` at parity you must know what the
bash does — and that knowledge *is* the inventory. It is a prerequisite of
writing the Rust command's tests, so it sits on the critical path to the command
whether or not deletion is bundled. Splitting deletion out would therefore move
almost nothing: the expensive, risky part (inventory + Rust tests) stays in story
one regardless, and story two would contain the `rm`, the counter decrement, and
the `mise run` check. That is not a story worth the coordination cost, and
carving it out would leave the epic with a low-visibility maintenance ticket that
predictably slips while orphaned readers sit in the tree.

**Two second-order reasons that reinforce it.** Keeping the scripts after the
cutover means CI runs 6,289 lines of shell tests against code nothing calls —
not neutral, but a false green signal and a maintenance tax. And an orphaned
reader is a live re-entry point: a future call site can be added against it, and
the passing suite would make that look fine.

**Mitigation, since the story is genuinely large.** The internal phase order is
what makes it trackable, and it is now stated rather than left implicit:
(1) probe Q1/Q2 and record the answers; (2) behaviour inventory + surviving-suite
audit; (3) `store` consolidation; (4) the `config` command and its Rust tests;
(5) the `configure` round-trip proof; (6) the call-site and glob cutover;
(7) hooks; (8) deletion, counter decrement, `mise run` green. Deletion is last,
so the cutover is *not* gated behind it — the ordering the objection assumed is
inverted.

**What would change this answer.** The research's cross-cutting test strategy
(Q7) says to *repoint* an existing shell suite at the new binary as a black-box
parity gate during cutover, retiring it in the same change that deletes the
scripts. 0167 instead states "the suites are deleted here, not repointed" — and
gives no reason. That choice is the sole origin of the 337-assertion inventory
burden: a repointed `test-config.sh` running against `accelerator config` *is* the
parity gate, and no inventory is needed. **This is worth revisiting before
implementation starts.** If repointing is viable, the dominant thread largely
evaporates and the scope objection dissolves rather than being answered. If it is
not viable — most likely because the suite asserts on bash-specific invocation
shapes that no longer exist — record why, because that reason is what justifies
the inventory's existence.

### Review pass 3 — editorial repairs (2026-07-19)

Repairs to text earlier passes introduced. No scope or design changes.

- **Resolved the exit-code collision**: the subcommand-surface criterion demanded
  exit 0 from `templates eject|diff|reset` while the ADR-0021 criterion demanded
  exit 2 from the same commands. The surface criterion now asserts each
  subcommand's *documented success code* against the fixture state its success
  path requires, and the ADR-0021 criterion explicitly overrides for the three
  mutation paths.
- **Partitioned the subcommand surface** into scalar / block / mutation output
  classes, so "each scalar read command" has an enumerated denominator instead of
  leaving `get`/`agent`/`template` undecidable, and "produces its documented
  output" points at a committed golden rather than nothing.
- **Widened the call-site denominator** to span every path in the removal-set file
  list. `init.sh` and the per-skill readers contain no `config-` segment, so a
  `grep 'scripts/config-'` alone would have reported success with their call sites
  unmigrated. Added the expected residual `config-read-browser-executor.sh` count
  so the post-migration check is numeric rather than per-hit judgement.
- **Replaced the "nineteen surviving suites" count with enumeration.** No literal
  is correct throughout: `test-init.sh` is wired in mid-story and retired later,
  so the registered population changes as the work proceeds. The audit table's row
  count and the discovered count must agree.
- **Gave the 337-assertion attribution a procedure** — keyed by
  `test-config.sh:<line>`, two-column mapping, committed script asserting no gaps
  or duplicates. A bare total can be reached with mis-attributions cancelling out.
- **Split the fail-open criterion in two.** The three injection blocks are separate
  commands and therefore separate processes, so "any healthy sibling block still
  renders" could not be observed within one invocation; per-source degradation is
  now its own criterion asserted across three. Also replaced "contains exactly"
  with byte equality plus trailing newline.
- **Made atomicity structural rather than raced** (injected filesystem port,
  recorded call sequence) — the prior wording forbade racing and then prescribed a
  100-iteration race that cannot conclusively fail.
- **Added a per-commit replay** for the same-commit browser-executor re-homing.
  Final-tree checks cannot distinguish a rule added in the right commit from one
  added three commits later, so the one requirement written to prevent a transient
  permission gap was unverifiable.
- **Fixed the latency criterion**: an upper bound (≤ 30 ms p95) rather than a band
  that a faster-than-bash result would fail; a self-relative bash baseline captured
  in the same run, so no undefined "reference machine"; a static crate-graph check
  in place of unobservable runtime "reachability"; and an explicit
  record-and-follow-up path on failure, so it cannot become an optimisation tail.
- **Made the superseded-suite assertion primary** over the `_EXPECTED_CONFIG_SUITES`
  equality, which is tautological once the constant is edited.
- **Named 0106's plan as the authoritative artefact** instead of deferring with
  "identify which and say so", and added a criterion recording Q2's answer (Q1 had
  one; Q2 was labelled blocking with no definition of done).
- Defined the build-system jargon (`run_shell_suites`, `_EXPECTED_CONFIG_SUITES`,
  "the eight subtrees") once in Technical Notes; marked the Q1 Assumptions slot as
  RESERVED so its unfilled state is unambiguous; corrected the Blocked-by prose,
  which still listed 0180 after it moved to `relates_to`; clarified the `blocks`
  antecedent; noted that the 44 `config-common.sh` sourcers shrink post-migration
  and both numbers should be recorded; reserved "removal set" as the precise term
  and "config cluster" for background prose; and moved the settled `--format`
  decision out of Open Questions so that list holds only blocking items.

### Review pass 3 — `atomic_write` settled (2026-07-19)

- **Corrected the premise before deciding.** The review, and the first framing of
  this decision, both assumed `atomic_write` was future work owned by 0180. It is
  not: 0178 is **done** and already shipped one in
  `cli/config-adapters/src/store.rs`. The real situation was never "who builds
  it" but "it exists once, and 0180 is about to build a second". Two related
  review findings dissolved on inspection — the feared config→corpus dependency
  edge does not arise (`config-adapters` already depends on the shared `document`
  crate, not on anything corpus-side), and the duplication was not an oversight:
  0166 reasoned about it explicitly and chose folding over a standalone crate.
- **Decision: extract into a standalone `store` crate anyway.** Consuming the
  existing implementation in place was the lower-friction option, but `store` is
  the crate name 0166 itself reserved, and its stated trigger — "a later split
  stays open if a second consumer needs it independently" — is precisely what
  `config set` is. Taking the split now avoids shipping the duplicate at all,
  rather than shipping it and deduplicating later.
- **This story owns the consolidation, not 0180.** An earlier draft delegated the
  extraction to 0180 and made its agreement a precondition — which reproduced the
  defect the review had just flagged: an unenforceable cross-item gate on a
  sibling that may proceed independently. Owning detection-and-extraction here
  makes the outcome ordering-independent (two implementations to consolidate if
  0180 lands first, one to extract if not), so 0180 needs no amendment, no scope
  expansion, and no negotiated consent. 0180 accordingly moved from `blocked_by`
  to `relates_to`; nothing in this story waits on it.
- **Scope limited to `atomic_write`.** 0180's mkdir-lock and JSONL primitives have
  corpus-only callers and are not duplicated, so they are left alone and `store`
  starts as a single-primitive crate. This also dissolves the "do all three move?"
  question the earlier framing raised — only the duplicated primitive moves.
- **The consolidation check outlives the story.** Requiring a committed check that
  fails on a reintroduced duplicate — validated known-positive against the
  pre-consolidation tree — turns a one-time tidy-up into a standing guarantee, and
  is what makes 0180 landing later harmless rather than a regression.
- **The symlink-escape refusal moves with the primitive**, generalised to a
  permitted-root parameter so corpus gets the same guarantee and no
  config-specific logic lands in a shared crate. The existing implementation has
  no such refusal — a real gap, not just a relocation.
- Recorded the knock-on work the extraction forces (workspace member, `pup.ron`
  rule for an infrastructure crate that domain crates still may not import,
  `deny.toml` check) and flagged the temp-file placement discrepancy: the shipped
  code uses a `tmp` *subdirectory* while 0180 specifies same-directory. Same
  filesystem in practice, so `rename` stays atomic — but a `tmp` on a different
  mount would silently degrade it to a copy, so the extracted primitive must pick
  one.
- Amending 0166's resolved decision is a consequence of this choice and must be
  recorded there. **0180's scope is not expanded** — this story owns detection
  and extraction, which is what makes the outcome ordering-independent. (An
  earlier note in pass 2 said otherwise; it is struck above.)

### Acceptance Criteria rewrite (2026-07-19)

The criteria had reached 32 entries through four rounds of patching, and each
round's repairs were producing fresh contradictions with text elsewhere (pass 4's
critical: widening the call-site grep — correct in isolation — broke the residual
criterion added in the same batch). The section was rebuilt in one pass rather
than patched further.

- **Grouped by theme** (surface, error posture, parity, suite lifecycle,
  invocation contract, hooks, `config set`/`store`, cross-item records, removal,
  performance), so related criteria sit together and a contradiction between two
  of them is visible rather than pages apart.
- **The repointing question is settled — repoint, inventory the remainder.**
  Evidence: `test-config.sh` binds each script path to a variable once (~20
  bindings) and invokes them uniformly as `bash "$VAR" args`, so redirection is
  mechanical. This supersedes the "deleted here, not repointed" position and
  **substantially shrinks what the scope objection was about** — the inventory now
  covers only the call-site greps, the `config-defaults.sh` file assertions, and
  removal-set scripts with no covering suite, rather than all 337 assertions. The
  answer recorded above still holds, but the thread it defends is much smaller.
- **Output classes fixed**: `templates diff` was in both `block` and "the three
  mutation paths". The class is renamed **customisation-state**
  (`eject|diff|reset`), distinct from **mutation** (`set|init`), and the ADR-0021
  criterion references it by name so the lists cannot drift. The escape-hatch
  clause ("fix the table if a class is wrong") is gone — a class change now
  requires amending the criterion.
- **The coverage script now extracts bare-path invocations too.** It previously
  read only `accelerator` invocations, so it never examined
  `config-read-browser-executor.sh` — meaning the per-commit replay could not
  verify the same-commit re-homing it was added to verify.
- **Latency made binding and self-relative**: p95 no greater than the bash p95
  captured in the same run on the same host. The absolute 30 ms figure is gone (it
  needed a reference machine the document never named, and would have failed a
  faster-than-bash result), as is the waiver criterion that made a miss
  consequence-free.
- **Criteria added for previously unasserted outputs**: the `test-init.sh`
  characterisation green run, the 0166 amendment, the reciprocal `blocked_by`
  edges on 0169-0174 (decided: add them, not one-sided convention), and a defined
  procedure for the live hook-equivalence capture.
- **Audit table pinned to a revision** with a recorded final-state run, since the
  discovered suite population changes mid-story and no single count holds
  throughout.
- Smaller repairs: the inventory cardinality recorded rather than
  asserted against 337 (a line-keyed extraction counts call sites, not
  assertions); "exactly one implementation" restated as "none outside `store`",
  which is what the check proves; fail-safe stated as the default rather than an
  unspecified mode; the three injection commands named by subcommand rather than
  by output heading; each criterion pointed at a named fixture.

### Confirmation pass and repairs (2026-07-19)

Clarity and testability were re-run against the rewritten section. They confirmed
the structural fixes held — output classes no longer overlap, fixtures resolve
one-to-one, the greps are consistent, and criteria previously satisfiable by
construction are now anchored to artefacts that can fail. They also found that
the rewrite **dropped the `configure` round-trip criterion entirely**, while the
rewrite's own note claimed it had been "narrowed to removal-set commands" — a
description of an edit that was never made. That note has been corrected and the
criterion restored as its own **End-to-end proof** group, now also asserting the
`config set` round trip through the skill, which nothing else covered outside unit
fixtures.

Other repairs from the same pass:

- **Closed the parity gap around `test-init.sh`.** The remainder was enumerated
  exhaustively as three members, none of which covered it: it is not repointed,
  and `init.sh` *has* a covering suite so it fell outside "scripts with no covering
  suite". The one suite that has never run in CI belonged to neither gate. It is
  now the explicit fourth member, and the two gates are stated as exhaustive.
- **Restored a depth floor** for the inventory's unbounded members (`test-init.sh`
  and uncovered scripts): every top-level control-flow branch and every distinct
  exit code is its own row. The rewrite had dropped pass 2's 337-assertion floor
  as no longer applicable without replacing it, leaving a single hand-wavy row per
  script sufficient to make it deletable.
- **Made two criteria satisfiable that were not.** Live hook equivalence compared
  the whole `--format=hook` envelope against the `additionalContext` field — a JSON
  object against one of its string values, which could never match; it now parses
  the field out first. And fail-open's antecedent ("given a config error") was a
  superset of the three fail-closed triggers, so an invalid `work.integration`
  value satisfied both criteria while they demanded opposite exit codes; fail-open
  now excludes them explicitly.
- **Fixed Grep A's undefined corpus.** Its expected value of exactly 0 is
  unreachable tree-wide — the removal-set paths appear in this work item and the
  research document — and a scope chosen at verification time could be narrowed
  until it passed. It now names the same literal corpus as Grep B.
- **Named the three fail-closed fixtures** (writeback-failure,
  bad-integration-enum, doc-type-escape) with their exercising commands, and added
  them to the Technical Notes list; softened the section preamble, which claimed
  every criterion names a fixture when the repository-acting groups name none.
- **Pinned the superseded-suite absence criterion to the final state**, since
  `test-init.sh` is deliberately wired *in* mid-story and the criterion read as an
  invariant.
- **Settled the reciprocal-edge scope**: 0169/0173/0174 only, matching `blocks`.
  The blanket "each of 0169-0174" would have created one-sided edges for
  0170-0172 — the precise asymmetry the criterion exists to remove — and
  Dependencies still posed the decision as open after it had been made.
- **Updated the stale Assumptions parity bullet**, which still asserted the
  superseded whole-removal-set inventory as the sole parity measure and would have
  led a reader straight back to the 337-row burden the repointing decision
  removed. Also dropped the dangling 337 comparison from the remainder criterion.

### Planning pass — corrections from implementation planning (2026-07-19)

Made while writing
`meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`,
which supersedes this item where the two differ on mechanics. Four of these are
defects no review pass caught; all were established by inspecting the tree at
revision `b290d5d9` rather than by argument.

- **The ADR-0021 exit-2 criterion was wrong and unsatisfiable.** It asserted exit
  2 from `eject|diff|reset` against a single **not-customised** fixture. But the
  three fire on *opposite* customisation states: `eject` exits 2 when the override
  **already exists** (`config-eject-template.sh:133-135`), while `diff` and
  `reset` exit 2 when there is **none** (`:36,43` and `:60,67`). Against
  not-customised, `eject` *succeeds*. Replaced with a per-command table and a new
  **already-customised** fixture. ADR-0021:80's actual definition — "destructive
  action requires confirmation" — fits `eject` and is overloaded by the other two.
- **Exit 2 collides with clap, a regression the port would introduce.** clap 4
  exits **2** on usage errors and `cli/launcher/src/main.rs:106` delegates to
  `error.exit()`. The bash exits **1**. Without interception, a mistyped template
  name is indistinguishable from "confirmation required" — which would silently
  corrupt the two-phase flow ADR-0021 exists to drive. New criterion added.
- **`doc-type-paths` was mis-classed `scalar`.** It emits 13 tab-separated
  `type<TAB>dir` lines under `LC_ALL=C` (`config-read-doc-type-paths.sh:81-110`),
  never a single value. Moved to **block**.
- **Q2 resolved and recorded in Context**, and struck from Open Questions. The gap
  is benign; nothing is broken today. Two files gain rules for the new bootstrap
  path — `vcs/commit` (whose `scripts/*` rule does not cover `bin/accelerator`)
  and `configure` (which has no `allowed-tools` key at all).
- **`_EXPECTED_CONFIG_SUITES` is a `<` floor, not an equality**
  (`tasks/test/integration.py:85`). The research called it "zero headroom", which
  is right about the consequence and wrong about the mechanism. Retiring suites
  requires *lowering* it either way.
- **`scripts/test-design.sh` added to the surviving-suite audit** — named nowhere
  in this item previously, but it asserts SKILL.md invocation shape in the same
  class as the flagged `test-config.sh` regions and breaks by design at cutover.
  The audit also now names the two Rust tests and two stub-writing suites that
  pin the shell surface.
- **The `store` duplication check needs an allowlist.** Of the four
  temp-and-rename sites, only two are duplicates. `cache.rs:112-127` is a 0600
  publication primitive with a paired signature file; `lock.rs:106-117` renames a
  *directory* as a lock claim. A naive `fs::rename` grep flags both forever.
- **Statuses corrected**: 0179 and 0180 have both landed. The "one if 0180 has not
  landed, two if it has" hedge resolves to **two**, and the pass-2 note claiming
  the decision "expands 0180's scope" is struck — pass 3 superseded it.
- **Three deliberate divergences from bash parity** are recorded in the plan and
  will be committed as a divergence note: `config_assert_no_legacy_layout` applied
  **uniformly** (the bash applies it to 7 of 20 scripts, asymmetrically and
  undocumented); the `config-read-review.sh:270` **double slash** in custom-lens
  paths **fixed** rather than frozen into a golden; and the
  `config-summary.sh:20-22` init sentinel resolved against **project root** rather
  than CWD. Each updates its repointed assertion in the same commit.

**Not addressed, deliberately.** The ~40-line "Answering the scope objection"
section still argues against the deleted-not-repointed premise that the AC rewrite
reversed. It is now historical rather than wrong — the section says so itself in
its final paragraph — so it is left standing as a record of how the decision moved.

## References

- Plan: `meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- Research: `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0047, ADR-0020, ADR-0021
- Related: `meta/work/0106-invoke-plugin-scripts-by-bare-path.md`, `meta/work/0107-lint-skill-body-script-invocations.md`
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0011-configuration-feature-parity-with-accelerator.md
- Interface reference (luminosity): work items 0016 (plugin-global context injection), 0017 (per-skill context injection), 0019 (template management subcommands, draft)
