---
type: "plan"
id: "2026-10-10-0303-research-concurrency-subagent-limit"
title: "Research Concurrency Within the Subagent Limit Implementation Plan"
date: "2026-10-10T21:22:58+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0303"
parent: "work-item:0303"
derived_from: ["codebase-research:2026-10-10-0303-research-concurrency-subagent-limit"]
relates_to: ["plan:2026-09-26-0283-recursive-finding-deepening", "plan:2026-09-20-0282-tunable-depth-and-breadth"]
tags: ["research", "research-topic", "conduct", "concurrency"]
revision: "6451d97d7135f0b04eb5a9d349d24a657dc36c6b"
repository: "accelerator"
last_updated: "2026-10-11T16:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research Concurrency Within the Subagent Limit Implementation Plan

## Overview

`research-topic conduct` offers batches of up to 24 spawns, but Claude Code
starts at most 20 subagents at once, so 4 spawns per full batch are refused
and lost. This plan lowers the default to 20 and moves concurrency
resolution into `accelerator research topic outstanding`. The CLI clamps the
value to the subagent cap, which it reads from `ACCELERATOR_SUBAGENT_CAP` or
Claude Code's `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, in one module the
coupling guard admits by variable name. The skill gains retry rounds for
spawns the harness refuses for capacity.

## Current State Analysis

- The default of 24 lives in `cli/config/src/catalogue.rs:393`. It is pinned
  in three places: the catalogue test (`catalogue.rs:540-543`),
  `cli/launcher/tests/fixtures/dump/dump.golden:18`, and the configure table
  (`skills/config/configure/SKILL.md:438`).
- The skill resolves concurrency in prose. It injects the configured value
  (`skills/research/research-topic/SKILL.md:67`), applies 0282's rule
  (`SKILL.md:69-87`, "There is no upper bound") and passes the result as
  `--limit {concurrency}` (`SKILL.md:245,253`).
- `outstanding` takes `--limit` as a raw string (`cli/research-cli/src/cli.rs:104-106`).
  `OutstandingFlags::parse` hard-fails a malformed value with
  `E_TOPIC_RESEARCH_LIMIT` (`topic_command.rs:113-124`), and the value
  reaches `research::conduct::window::window` as `limit: Option<usize>`,
  where `None` means unlimited (`window.rs:51`).
- `outstanding` reads no research knob from config today. It already holds
  the composed config through `ProjectContext.config`
  (`cli/research-cli/src/context.rs:14-32`).
- Step 4 issues a batch in one message and has no rule for a refused Task
  call. Step 5 lists "a refusal" among its no-file reasons
  (`SKILL.md:361-364`). Step 6 already records only `unfinished` spawns
  "not already recorded as failed" (`SKILL.md:374`).

## Desired End State

- With default config and no `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, every
  `outstanding` plan offers at most 20 spawns and reports `concurrency: 20`.
- `outstanding --concurrency N`, or `research.topic.concurrency` when no
  flag is given, is validated and then clamped to the harness cap. The cap
  is the first of `ACCELERATOR_SUBAGENT_CAP` and
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` holding a value the harness would
  honour, each read as the harness reads its own setting, or 20 when neither
  does. The CLI warns on `--start` when a flag or config value was clamped,
  naming `--concurrency` or `research.topic.concurrency` as the value's
  source.
- Every plan, a plain query included, offers at most the resolved
  concurrency. The spawn window takes a `Concurrency`, which code outside
  the research crate can obtain only by resolving against the cap.
- `conduct` passes the invocation's `--concurrency` through to every plan,
  the tick plan included, with no shell variable in any command, and
  re-issues capacity-refused spawns in rounds before checking them.
- `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` is the only `CLAUDE_*` name under
  `cli/`, and appears only in its reader and that reader's integration test.
  `lint:claude-coupling:check` enforces this by variable name, and ADR-0071
  records why.
- `mise run` exits 0.

Verify with the Rust and structural tests named in each phase, plus the
manual runs in Phase 4 and Phase 5.

### Key Discoveries:

- **Provenance:** `ConfigAccess::effective(&key, None)` returns a
  `Resolution` whose `is_from_config()` separates a team or personal value
  from the built-in default (`cli/config/src/service.rs:71-99,433-454`).
  `effective`, unlike `effective_nonempty`, keeps an explicit empty value,
  so the empty-value error stays reachable.
- **Coupling guard:** `lint:claude-coupling:check`
  (`tasks/lint/claude_coupling.py`) fails on any `CLAUDE_*` text under
  `cli/**` or in `bin/accelerator`, with no file allowlist by design. Its
  invariant is that no entry point may *require* a `CLAUDE_*` variable: it
  was written after a launcher-layer read of `CLAUDE_PLUGIN_ROOT`, which the
  Bash tool never receives, broke every CLI-invoking skill. An optional
  tuning variable with a default does not share that failure mode, so the
  guard can admit one by name, confined to its reader, without a file
  exemption.
- **Variable expansion prompts:** Claude Code 2.1.296 refuses to auto-allow
  any Bash command containing a shell variable, whatever the skill grants.
  Under `--permission-mode default` with only
  `Bash(accelerator research topic *)` allowed, `--subagent-cap 8` ran, while
  `--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"` and its
  `${…:-}` form were both blocked: "A variable in this command can't be
  checked before it runs". Phase 3 shipped that form, so every interactive
  `conduct` plan prompted until Phase 4 replaced it.
- **Environment reaches the CLI:** a cap exported in the shell that launches
  Claude Code, or set under `env` in the project `settings.json`, the project
  `settings.local.json` or the user `settings.json`, reaches
  `accelerator research` through the Bash tool (Phase 4's manual runs,
  `concurrency` 8 each, Claude Code 2.1.296).
- **Why not `!`:** the skill permission lint (`tasks/lint/skill_permissions.py`)
  bans shell metacharacters in a `!` command. A `!` read of an unset
  variable also fails, discarding the prompt, so the cap cannot be injected
  at invocation time.
- **Composition-root env reads:** `linear-cli/src/context.rs:51` reads
  `ACCELERATOR_LINEAR_API_URL` with `std::env::var_os` in the composition
  root, not an adapter crate. The cap reader follows it, in `research-cli`.
- **Warning precedent:** the replaced-ledger notice is the precedent for a
  warning raised on `--start` only. It is gated on `RunMode::Start`, lands
  in both the JSON `warnings` and stderr (`topic_command.rs:225-228,267-278`),
  and is pinned by `starting_over_a_stale_ledger_replaces_it_with_a_warning`
  (`topic_outstanding.rs:941`).
- **Model types:** `Depth` (`cli/research/src/topic/tree.rs:19-43`) and
  `Limit` (`cli/research/src/sources/request.rs:163-205`) are the newtype
  models. A new public type changes `cli/research/tests/fixtures/public-api.txt`.
- **Config fixtures:** config levels are read from YAML frontmatter, so a
  test config must be fenced in `---`, as `support::config_with` does
  (`cli/research-cli/tests/support/mod.rs:321-323`). The loader ignores a
  `config.local.md` readable by group or others, so a test's personal
  config must be written with mode `0o600`, as `support::personal_config`
  lets its callers choose.
- **Whole-plan assertions:** three `topic_outstanding.rs` tests compare the
  entire plan object (`a_multi_profile_set_leaves_only_its_quarantined_pair_outstanding`,
  `a_quarantined_single_profile_pair_reuses_its_markers_index`,
  `a_legacy_finding_is_answered_and_shallow_at_depth_three`), so a new
  `concurrency` field breaks them.
- **Window callers:** `window` has one production caller
  (`topic_command.rs:252`). Its `None` limit is passed only by unit tests in
  `window.rs` and `ledger.rs`.
- **Ticking is unaffected:** `items[].complete` comes from `RoundPlan`, not
  the window (`topic_command.rs` `PlanOutput::render`), so capping a plain
  query does not change ticking.
- **Retry window:** a spawn re-issued before `--spawned` is acknowledged is
  still in the ledger's pending batch. Its file is excused from
  `unexpected` (`cli/research/src/conduct/window.rs:147-168`), so retry
  rounds need no Rust change.

## What We're NOT Doing

- Moving breadth or depth resolution into the CLI. They stay in prose, and
  `--depth` is still never read from config.
- Persisting the resolved concurrency in the run ledger. Each plan
  re-resolves it, which can change batch size if config changes mid-run.
  That is harmless.
- Adding a failure command or a reason field to the ledger. Failure reasons
  stay in the orchestrator's conversational tally.
- Stopping retry rounds early on a halting failure. That is 0310's rule;
  whichever of 0303 and 0310 lands second adds it.
- Making 0307's node query answer for re-issued spawns. 0307 owns that.
- Moving the 0282 knob rule for breadth and depth into Rust. The rule now
  has two homes: Rust is authoritative for concurrency, skill prose for
  breadth and depth. A structural test keeps their invalid-value wording
  in step.
- Reading any other agent's cap variable. The reader's table starts with
  Claude Code's alone; another agent's name is added only once that agent
  documents one.
- Exempting a file from the coupling guard. It admits named variables, each
  in its reader, and nothing else.
- Hand-editing the generated docs-site skill mirrors under
  `docs-site/src/content/docs/reference/skills/`, which are gitignored.

## Implementation Approach

The work is split into five phases, and each leaves `mise run` green:

1. Lower the default. This alone fixes the default-config case.
2. Add the concurrency rule to the research domain as a pure, fully
   unit-tested library type with no caller.
3. Wire the rule into `outstanding`, pass `Concurrency` into the spawn
   window, rename `--limit` to `--concurrency`, add `--subagent-cap`, and
   switch the skill to pass both flags through. These land together,
   because the flag rename and the skill's use of the flag break each
   other if split.
4. Move the cap into the CLI: read it from the environment, admit the one
   `CLAUDE_*` name in the coupling guard, record the decision as ADR-0071,
   and drop `--subagent-cap` from the CLI and the skill. This replaces
   Phase 3's flag transport, which the permission checker blocks.
5. Add the retry rounds to skill prose.

Every phase starts with failing tests. The Rust phases are driven by unit
and integration tests, and the prose phases by structural tests in
`tests/unit/tasks/test_research_structure.py`.

## Phase 1: Default of 20

### Overview

Lower the built-in default from 24 to 20 everywhere it is pinned.

### Changes Required:

#### 1. Catalogue test (red)

**File**: `cli/config/src/catalogue.rs`
**Changes**: In `default_for_the_research_knobs_are_typed_scalars`, assert
the concurrency default is `"20"`.

```rust
assert_eq!(
    default_for("research.topic.concurrency"),
    Some(Value::Scalar(Scalar::String("20".to_owned())))
);
```

#### 2. Dump golden (red)

**File**: `cli/launcher/tests/fixtures/dump/dump.golden`
**Changes**: Line 18 becomes `` | `research.topic.concurrency` | `20` | default | ``.

#### 3. Configure default pin (red)

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**: Add a `CONFIGURE` constant and a test over the configure
skill's research section.

```python
CONFIGURE = REPO_ROOT / "skills/config/configure/SKILL.md"


def _configure_research_section() -> str:
    return _section(
        _prose(CONFIGURE),
        "### research",
        "Example configuration",
        path=CONFIGURE,
    )


def test_configure_documents_a_concurrency_default_of_20() -> None:
    assert "| `concurrency` | `20` |" in _configure_research_section()
```

`_section` names `RESEARCH_TOPIC` in its assertion messages. Give it a
`path` parameter defaulting to `RESEARCH_TOPIC`, so a missing marker names
the right file.

#### 4. Default (green)

**File**: `cli/config/src/catalogue.rs:393`
**Changes**: `("research.topic.concurrency", Default::Scalar("20"))`.

**File**: `skills/config/configure/SKILL.md:438`
**Changes**: The `concurrency` row's default becomes `` `20` ``.

**File**: `CHANGELOG.md:139-142`
**Changes**: Amend the unreleased entry in place: "(default 20)".

### Success Criteria:

#### Automated Verification:

- [x] The catalogue test passes: `cargo nextest run -p config default_for_the_research_knobs_are_typed_scalars` (from `cli/`)
- [x] The golden matches: `cargo nextest run -p accelerator dump_matches_the_committed_golden` (from `cli/`)
- [x] The configure pin passes: `uv run pytest tests/unit/tasks/test_research_structure.py -k concurrency_default`
- [x] `mise run check` exits 0
- [x] `mise run` exits 0

#### Manual Verification:

- [x] `accelerator config get research.topic.concurrency` in a project with no override prints `20`

---

## Phase 2: The Concurrency Rule in the Research Domain

### Overview

Add `cli/research/src/conduct/concurrency.rs`, a pure model of how a
requested concurrency becomes the number of agents `conduct` may spawn at
once:
- the harness's cap parser, as `SubagentCap`;
- 0282's positive-integer rule;
- the cap clamp, aware of where the value came from.

Nothing calls it yet.

### Changes Required:

#### 1. Domain tests (red)

**File**: `cli/research/tests/concurrency.rs` (new)
**Changes**: Black-box tests over the public API, following
`cli/research/tests/request.rs`. Each table-driven test loops over its
cases and names the failing case in its assertion message. A warning or
error carries the provenance of the value it names.

| Test | Cases | Expected |
|---|---|---|
| `an_unset_cap_is_the_harness_default_of_20` | `None` | 20 |
| `a_cap_is_parsed_as_the_harness_parses_it` | `"8"`, `"007"`, `"+8"`, `" 8 "`, `"\t8\n"`, `"\u{FEFF}8"`, `"30"`, `"1"`, 40 zeros then `"8"` | 8, 7, 8, 8, 8, 8, 30, 1, 8 |
| `a_cap_the_harness_ignores_is_the_default` | `"0"`, `"-5"`, `"-0"`, `"-"`, `"abc"`, `"20.5"`, `"1e2"`, `"1,000"`, `"٨"`, `"\u{85}8"`, `""`, `"  "`, `"+"` | 20 |
| `a_cap_too_large_for_u32_saturates` | `"99999999999"` | `u32::MAX` |
| `a_valid_request_at_or_below_the_cap_passes_unwarned` | `"20"`/config at cap 20, `"24"`/config at cap 30, `"1"`/flag at cap 20, `"020"`/flag at cap 20 | unchanged, no warnings |
| `a_request_above_the_cap_clamps_with_a_warning_when_set_explicitly` | `"24"`/config, `"50"`/flag at cap 20 | 20, one `AboveCap { given, provenance, cap }` |
| `the_built_in_default_clamps_silently` | `"20"`/default at cap 8 | 8, no warnings |
| `a_malformed_request_clamps_to_1_before_the_cap_applies` | `"0"`, `"-3"`, `"50.5"`, `"many"`, `"+5"`, `" 5"`, `"٥"` (flag); `"0"`, `"many"` (config) at cap 20 | 1, one `NotAPositiveInteger { given, provenance }`, no `AboveCap` |
| `an_empty_request_is_refused` | `""` from flag and from config | `Err(EmptyConcurrency { provenance })` |
| `a_request_too_large_for_u32_still_clamps_naming_what_was_given` | `"99999999999"`/flag at cap 20 | 20, `AboveCap` with `given` `"99999999999"` |
| `a_concurrency_admits_at_most_its_agents` | resolved `"20"`/flag at cap 20, admitting 25 and 5 waiting; resolved `"99999999999"`/flag at cap `"99999999999"`, admitting 25 | 20, 5, 25 |

#### 2. Domain model (green)

**File**: `cli/research/src/conduct/concurrency.rs` (new), declared in
`cli/research/src/conduct.rs` as `pub mod concurrency;`

```rust
//! How many agents `conduct` spawns at once: the requested concurrency,
//! validated, then bounded by the harness's subagent cap.

use std::num::NonZeroU32;

/// How many subagents the harness runs at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubagentCap(NonZeroU32);

impl SubagentCap {
    pub const HARNESS_DEFAULT: Self = match NonZeroU32::new(20) {
        Some(slots) => Self(slots),
        None => Self(NonZeroU32::MIN),
    };

    /// Reads the harness setting as the harness's JavaScript parser does:
    /// trimmed as `String.prototype.trim` trims, an optionally signed run
    /// of ASCII digits, at least 1; anything else leaves the default.
    #[must_use]
    pub fn from_setting(setting: Option<&str>) -> Self {
        setting
            .and_then(harness_slots)
            .map_or(Self::HARNESS_DEFAULT, Self)
    }

    #[must_use]
    pub const fn slots(self) -> NonZeroU32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    Flag,
    ConfigFile,
    BuiltInDefault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedConcurrency {
    pub value: String,
    pub provenance: Provenance,
}

/// The most agents `conduct` spawns at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concurrency(NonZeroU32);

impl Concurrency {
    #[must_use]
    pub(crate) const fn new(agents: NonZeroU32) -> Self {
        Self(agents)
    }

    #[must_use]
    pub const fn agents(self) -> NonZeroU32 {
        self.0
    }

    #[must_use]
    pub fn admits(self, waiting: usize) -> usize {
        usize::try_from(self.0.get())
            .map_or(waiting, |agents| waiting.min(agents))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcurrencyWarning {
    NotAPositiveInteger {
        given: String,
        provenance: Provenance,
    },
    AboveCap {
        given: String,
        provenance: Provenance,
        cap: SubagentCap,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConcurrency {
    pub concurrency: Concurrency,
    pub warnings: Vec<ConcurrencyWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyConcurrency {
    pub provenance: Provenance,
}

impl RequestedConcurrency {
    /// # Errors
    ///
    /// `EmptyConcurrency` when no value was given, which no clamp can
    /// stand in for.
    pub fn resolve(
        &self,
        cap: SubagentCap,
    ) -> Result<ResolvedConcurrency, EmptyConcurrency> {
        if self.value.is_empty() {
            return Err(EmptyConcurrency {
                provenance: self.provenance,
            });
        }
        let mut warnings = Vec::new();
        let valid = positive_integer(&self.value);
        if valid.is_none() {
            warnings.push(ConcurrencyWarning::NotAPositiveInteger {
                given: self.value.clone(),
                provenance: self.provenance,
            });
        }
        let requested = valid.unwrap_or(NonZeroU32::MIN);
        let agents = requested.min(cap.slots());
        if agents < requested && self.provenance != Provenance::BuiltInDefault
        {
            warnings.push(ConcurrencyWarning::AboveCap {
                given: self.value.clone(),
                provenance: self.provenance,
                cap,
            });
        }
        Ok(ResolvedConcurrency {
            concurrency: Concurrency::new(agents),
            warnings,
        })
    }
}
```

Three private helpers parse the values:
- `saturating_digits(&str) -> Option<u32>` accepts only a non-empty run of
  `u8::is_ascii_digit` bytes. It folds them with `saturating_mul` and
  `saturating_add`, so a long zero-padded value keeps its small value and
  only a true overflow saturates to `u32::MAX`.
- `positive_integer(&str) -> Option<NonZeroU32>` is `saturating_digits`
  above 0. It is the knob's grammar: no sign and no whitespace, so `" 5"`
  and `"+5"` are malformed, as they were for `--limit`. Unlike the CLI's
  `positive`, which rejects overflow, it saturates, so an over-large
  request clamps to the cap rather than to 1.
- `harness_slots(&str) -> Option<NonZeroU32>` is the cap's grammar. It
  trims as JavaScript does: `char::is_whitespace` except U+0085, which
  JavaScript keeps, plus U+FEFF, which `str::trim` keeps but JavaScript
  strips. A leading `-` yields `None`, because the harness
  ignores any value below 1. A leading `+` is dropped, and the rest goes to
  `positive_integer`.

#### 3. Public API snapshot

**File**: `cli/research/tests/fixtures/public-api.txt`
**Changes**: Regenerate with `mise run public-api:update`. The diff adds
only the `research::conduct::concurrency` items.

### Success Criteria:

#### Automated Verification:

- [x] Domain tests pass: `cargo nextest run -p research --test concurrency` (from `cli/`)
- [x] The public API snapshot matches: `mise run public-api:check`
- [x] `mise run cli:check` exits 0
- [x] `mise run` exits 0

#### Manual Verification:

- [x] Review the case table against the work item's parser and clamp criteria; every case in the work item appears

---

## Phase 3: `outstanding` Resolves Concurrency

### Overview

Replace `--limit` with `--concurrency` on `outstanding`, and add
`--subagent-cap`. The CLI resolves the flag or configured value through the
Phase 2 rule against the cap that `--subagent-cap` carries, read by
`SubagentCap::from_setting`. Phase 4 replaces this flag transport: Claude
Code blocks a Bash command that expands a shell variable. The spawn window takes the resulting
`Concurrency`. The CLI reports the result as `concurrency`, and emits
concurrency warnings on `--start`. The skill stops resolving concurrency
and passes both flags through, expanding
`$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` into `--subagent-cap`. The configure docs, the
`outstanding` reference and the changelog follow.

### Changes Required:

#### 1. Config writers (refactor, green before red)

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**: The cap reaches the CLI only as `--subagent-cap`, which tests
pass as an ordinary argument, so the harness needs no environment handling.
Add two config writers, each fencing its body as frontmatter:

```rust
fn configure_team_concurrency(
    &self,
    value: &str,
) -> Result<(), TestError> {
    self.write(".accelerator/config.md", &concurrency_config(value))
}

fn configure_personal_concurrency(
    &self,
    value: &str,
) -> Result<(), TestError> {
    self.write(".accelerator/config.local.md", &concurrency_config(value))
}

fn concurrency_config(value: &str) -> String {
    format!("---\nresearch:\n  topic:\n    concurrency: {value}\n---\n")
}
```

#### 2. Integration tests (red)

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**:

- **Renames to `--concurrency`:**
  - `a_limited_plan_offers_the_first_spawns_and_counts_the_rest` becomes
    `a_concurrency_of_2_offers_the_first_spawns_and_counts_the_rest`.
  - `a_plan_without_start_or_run_writes_no_ledger` passes `--concurrency 1`.
  - `a_note_written_mid_run_without_being_offered_is_reported_unexpected`
    passes `--concurrency 1`.
- **Trimmed cases:** `a_malformed_limit_or_run_exits_1` drops its three
  `--limit` cases and becomes `a_malformed_run_or_spawned_exits_1` with 5
  cases.
- **Whole-plan assertions:** the three tests named under Key Discoveries
  gain `"concurrency": 20` beside `"depth"`.
- **Replaced test:** `a_24_limit_offers_24_arxiv_spawns_and_counts_the_rest`
  becomes `a_run_offers_20_arxiv_spawns_by_default_and_counts_the_rest`. It
  passes no concurrency, expects 20 spawns, `remaining` 5 and
  `concurrency` 20.
- **New tests.** Each asserts `plan["concurrency"]` and the exact
  `plan["warnings"]`. Where a warning is expected, it asserts stderr's
  exact lines, each `warning: {warning}`; collecting them into one string
  with `format!` trips clippy's `format_collect`. Where none is, it asserts stderr carries no line naming
  `--concurrency` or `research.topic.concurrency`. Tests marked † run on
  the 25-item arXiv set and also assert the offered spawns and `remaining`.

| Test | Given | Expected |
|---|---|---|
| `a_configured_concurrency_above_the_cap_clamps_with_a_warning` † | config 24, `--start` | 20 offered, `remaining` 5; config clamp warning naming 24 and 20 |
| `a_configured_concurrency_at_the_cap_is_not_clamped` | config 20, `--start` | 20; no warnings |
| `a_raised_cap_admits_a_higher_concurrency` † | `--subagent-cap 30`, config 24, `--start` | 24 offered, `remaining` 1; no warnings |
| `a_lowered_cap_clamps_the_default_silently` † | `--subagent-cap 8`, `--start` | 8 offered, `remaining` 17; no warnings |
| `a_concurrency_flag_wins_over_config_and_clamps_with_a_warning` | config 6, `--concurrency 50 --start` | 20; flag clamp warning naming 50 and 20 |
| `a_personal_concurrency_wins_over_the_team_and_warns` | team 6, personal 24, `--start` | 20; config clamp warning naming 24 and 20 |
| `the_cap_is_read_as_the_harness_reads_it` | `--subagent-cap` `007`, `+8`, ` 8 `; `--start` | 7, 8, 8; no warnings |
| `a_cap_the_harness_ignores_leaves_the_default_unwarned` | `--subagent-cap` `0`, `-5`, `abc`, `20.5`, `1e2`, `1,000`, empty; `--start` | 20; no warnings; `-5` is taken as the value, not a flag |
| `a_cap_the_harness_ignores_still_clamps_a_configured_concurrency` | same caps, config 24, `--start` | 20; config clamp warning naming 24 and 20 |
| `a_malformed_concurrency_flag_clamps_to_1_with_a_warning` † | `--concurrency 0`, `--concurrency 50.5`; `--start` | 1 offered, `remaining` 24; only the flag invalid-value warning |
| `a_malformed_configured_concurrency_clamps_to_1_with_a_warning` | config `0`, config `many`; `--start` | 1; only the config invalid-value warning |
| `an_empty_concurrency_exits_1_and_writes_no_ledger` | `concurrency: ""` in config, `--concurrency ""`; each with `--start` and as a plain query | exit 1, the matching empty-value error, empty stdout, no ledger |
| `a_concurrency_flag_stands_in_for_an_empty_configured_value` | `concurrency: ""` in config, `--concurrency 5`; with `--start` and as a plain query | 5; no warnings |
| `an_unreadable_config_exits_1` | `.accelerator/config.md` with unterminated frontmatter | exit 1, non-empty stderr, empty stdout; green on arrival, since `working_project` already composes config. The config error's wording is the config crate's, so it is not pinned here |
| `concurrency_warnings_precede_a_replaced_ledger_notice` | config 24, a stale ledger, `--start` | the clamp warning then the replaced-ledger notice, in `warnings` and stderr |
| `concurrency_warnings_are_raised_only_when_a_run_starts` † | config 24; `--run R --spawned 0` after a start, and a plain query | continuation: 5 offered, `remaining` 0, 20 `unfinished`; plain query: 20 offered, `remaining` 5; no concurrency warning in either |
| `limit_is_no_longer_an_outstanding_flag` | a valid set, `--limit 2` | exit 2, stderr names `--limit` as unexpected |

The diagnostics name `{source}`: `--concurrency` for a flag value,
otherwise `research.topic.concurrency`.

```text
{source} is {given}, above the subagent cap of {cap} — clamping to {cap}
{source} must be a positive integer, got '{given}' — clamping to 1
E_TOPIC_RESEARCH_CONCURRENCY: --concurrency is empty; pass a positive integer
E_TOPIC_RESEARCH_CONCURRENCY: research.topic.concurrency is empty; set it to a positive integer or remove it
```

**File**: `cli/research-cli/src/plan_wording.rs`
**Changes**: Add `every_concurrency_diagnostic_names_its_source` to the
`tests` module. It covers both warning variants from each explicit
provenance, and the empty-value clause from each (the error text after
`E_TOPIC_RESEARCH_CONCURRENCY: `), against the strings above. Wrap long
expected strings with `\`-continuations, as the sibling tests do.

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: Add a `tests` module. Its cases pin precedence and the cap:
- `a_concurrency_flag_never_reads_the_configured_value`: a flag of `5`
  with a configured fallback that returns `Err` resolves to 5;
- `without_a_flag_the_configured_value_is_resolved`: a configured `24`
  under a cap of `30` resolves to 24;
- `the_subagent_cap_flag_bounds_the_configured_value`: `parse` given a
  `--subagent-cap` of `8`, then resolving a configured `24`, yields 8 with
  exactly `[AboveCap { given: "24", provenance: ConfigFile, cap:
  SubagentCap::from_setting(Some("8")) }]`;
- `without_a_subagent_cap_flag_the_cap_is_the_harness_default`: `parse`
  given no `--subagent-cap` records `SubagentCap::HARNESS_DEFAULT`;
- `an_empty_concurrency_names_its_source_in_the_error`: an empty flag
  yields `E_TOPIC_RESEARCH_CONCURRENCY: --concurrency is empty; pass a
  positive integer`;
- `a_configured_value_that_cannot_be_read_is_an_error`: no flag and a
  fallback returning `Err("unreadable")` yields that error.

**File**: `cli/research-cli/src/context.rs`
**Changes**: Add a `tests` module with
`the_built_in_default_sits_at_the_harness_cap`. It asserts that
`config::catalogue::default_for("research.topic.concurrency")` renders
`SubagentCap::HARNESS_DEFAULT.slots()`.

**File**: `cli/research/src/conduct/window.rs`, `cli/research/src/conduct/ledger.rs`
**Changes**: The window and ledger unit tests pass a `Concurrency`. A test
that passed `Some(n)` passes `concurrency(n)`, and one that passed `None`
passes `unbounded()`, which is `Concurrency::new(NonZeroU32::MAX)`. Both
helpers live once, as `pub` functions in a
`#[cfg(test)] pub(crate) mod test_support` in
`cli/research/src/conduct/concurrency.rs`, which the window and ledger
test modules import. Clippy's `redundant_pub_crate` rejects `pub(crate)`
items inside the crate-private module. `concurrency(n: u32)` builds its `NonZeroU32` with
`unwrap_or(NonZeroU32::MIN)`, as `plan_wording.rs:176` does, because
`unwrap_used` applies to test code too. The
retired `limit` vocabulary goes with it:
- the `unwatched` and `offered` helpers take `concurrency: Concurrency`;
- `the_window_holds_back_spawns_past_its_limit_and_counts_them` becomes
  `the_window_holds_back_spawns_past_its_concurrency_and_counts_them`;
- `a_limit_splitting_a_pairs_nodes_keeps_its_first_nodes` becomes
  `a_concurrency_splitting_a_pairs_nodes_keeps_its_first_nodes`;
- `a_pair_whose_nodes_all_fall_past_the_limit_is_not_offered` becomes
  `a_pair_whose_nodes_all_fall_past_the_concurrency_is_not_offered`;
- `a_window_without_memory_or_limit_offers_every_pair` becomes
  `a_window_without_memory_at_unbounded_concurrency_offers_every_pair`.

`rg -n limit cli/research/src/conduct/window.rs` prints nothing once the
phase is green.

#### 3. Flag and parsing (green)

**File**: `cli/research-cli/src/cli.rs`
**Changes**: Replace the `limit` field.

```rust
/// The most spawns to offer; the rest are counted in `remaining`.
/// Defaults to `research.topic.concurrency`, and is clamped to
/// `--subagent-cap`.
#[arg(long, allow_hyphen_values = true)]
concurrency: Option<String>,
/// How many subagents the harness runs at once, read as the harness
/// reads its own setting; a value it would ignore, or none, means 20.
#[arg(long, allow_hyphen_values = true)]
subagent_cap: Option<String>,
```

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: `OutstandingFlags.limit: Option<usize>` becomes
`requested_concurrency: Option<RequestedConcurrency>`, and the flags gain
`subagent_cap: SubagentCap`. `parse` takes `concurrency: Option<&str>` and
records it with `Provenance::Flag`, unvalidated, because validation needs
the cap and the config fallback. It also takes `subagent_cap:
Option<&str>` and reads it with `SubagentCap::from_setting`, which never
fails: a value the harness would ignore is ignored here too, silently.
Remove the `E_TOPIC_RESEARCH_LIMIT` branch.

#### 4. Resolution and output (green)

**File**: `cli/research/src/conduct/window.rs`
**Changes**: `window` takes `concurrency: Concurrency` in place of
`limit: Option<usize>`. The offered count becomes
`concurrency.admits(fresh.len())`, and the doc comment says "the first
spawns the concurrency admits".

**File**: `cli/research-cli/src/context.rs`
**Changes**: Add the configured read beside `research_scratch`.

```rust
impl ProjectContext {
    /// The configured concurrency and whether a config file set it.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when the key cannot be resolved.
    pub fn configured_concurrency(
        &self,
    ) -> Result<RequestedConcurrency, ConfigError> {
        let key = Key::parse("research.topic.concurrency")?;
        let resolution = self.config.effective(&key, None)?;
        let provenance = if resolution.is_from_config() {
            Provenance::ConfigFile
        } else {
            Provenance::BuiltInDefault
        };
        Ok(RequestedConcurrency {
            value: resolution.rendered(),
            provenance,
        })
    }
}
```

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**: Add the resolver, which owns precedence and the rule, and
thread its result through `run_outstanding` in place of `limit`.

```rust
/// # Errors
///
/// A message when the configured value cannot be read, or when the
/// requested concurrency is empty.
pub fn resolve_concurrency(
    flag: Option<RequestedConcurrency>,
    configured: impl FnOnce() -> Result<RequestedConcurrency, String>,
    cap: SubagentCap,
) -> Result<ResolvedConcurrency, String> {
    let requested = match flag {
        Some(requested) => requested,
        None => configured()?,
    };
    requested.resolve(cap).map_err(|empty| {
        failed(format!(
            "E_TOPIC_RESEARCH_CONCURRENCY: {}",
            plan_wording::empty_concurrency(empty)
        ))
    })
}
```

In `run_outstanding`:
- the `limit: Option<usize>` parameter becomes
  `resolved: &ResolvedConcurrency`;
- the `RunMode::Start` arm extends `notices` with
  `resolved.warnings.iter().map(plan_wording::concurrency_warning)` ahead
  of the replaced-ledger notice;
- `window` receives `resolved.concurrency`;
- `PlanOutput` gains `concurrency: Concurrency` and renders
  `"concurrency": self.concurrency.agents().get()` beside `"depth"`.

**File**: `cli/research-cli/src/plan_wording.rs`
**Changes**: Add `concurrency_warning(&ConcurrencyWarning) -> String` and
`empty_concurrency(EmptyConcurrency) -> String`, producing the warnings
and the empty-value clause above. A private
`concurrency_source(Provenance)` maps `Flag` to `--concurrency` and every
other provenance to `research.topic.concurrency`. The module doc gains
"and how a resolved concurrency's diagnostics read".

**File**: `cli/research-cli/src/main.rs`
**Changes**: `topic` passes `concurrency.as_deref()` and
`subagent_cap.as_deref()` to `parse`. `outstanding` resolves concurrency
after `working_project`:

```rust
let resolved = topic_command::resolve_concurrency(
    flags.requested_concurrency,
    || {
        project
            .configured_concurrency()
            .map_err(|error| error.to_string())
    },
    flags.subagent_cap,
)?;
```

It then passes `&resolved` to `run_outstanding`.

**File**: `cli/research/tests/fixtures/public-api.txt`
**Changes**: Regenerate with `mise run public-api:update`. The diff
replaces only the `research::conduct::window::window` line, whose
`limit: core::option::Option<usize>` becomes
`concurrency: research::conduct::concurrency::Concurrency`.

#### 5. Skill passes the flags through (red, then green)

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**: Update `test_research_topic_batches_through_the_spawn_window`,
delete `test_research_topic_plans_each_batch_at_the_resolved_depth` and
`test_research_topic_clamps_concurrency_under_the_knob_rule`, and add five
tests (red). The updated spawn-window test carries both step strings the
deleted depth test pinned, and the new knob test carries the surviving
assertion of the deleted clamp test.

```python
CONCURRENCY_FLAG = "{concurrency flag}"
SUBAGENT_CAP_FLAG = '--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"'
PLAN_AT_DEPTH = f"--depth {{depth}} {CONCURRENCY_FLAG} {SUBAGENT_CAP_FLAG}"
PLAN_WORDING = REPO_ROOT / "cli/research-cli/src/plan_wording.rs"


def _conduct() -> str:
    return _section(_research_topic_prose(), "### conduct", "### synthesise")


def test_research_topic_batches_through_the_spawn_window() -> None:
    conduct = _conduct()
    for step in (
        f"{PLAN_AT_DEPTH} --start",
        f"{PLAN_AT_DEPTH} --run {{run}} --spawned {{batch}}",
    ):
        assert step in conduct, (
            f"conduct must plan through the spawn window with {step!r}"
        )
    assert "--limit" not in conduct, "outstanding no longer takes --limit"
    # ... unchanged end-run / manifest ordering assertions


def test_research_topic_ticks_with_the_invocations_concurrency() -> None:
    tick = _section(_conduct(), "**Tick from disk.**", "**Edit `manifest.md`**")
    assert PLAN_AT_DEPTH in tick, (
        "the tick plan must resolve concurrency as every other plan does"
    )


def test_research_topic_passes_the_concurrency_flag_through() -> None:
    conduct = _conduct()
    assert (
        "Set `{concurrency flag}` to `--concurrency 'N'` when the invocation "
        "carries `--concurrency N`, and to nothing otherwise."
    ) in conduct
    assert (
        "report the first plan's `warnings` as soon as it returns — a "
        "replaced ledger or a clamped concurrency"
    ) in conduct


def test_every_plan_passes_the_shell_expanded_subagent_cap() -> None:
    plans = [
        line
        for line in _read(RESEARCH_TOPIC).splitlines()
        if line.lstrip().startswith("accelerator research topic outstanding")
    ]
    assert plans, "research-topic must plan through outstanding"
    for plan in plans:
        assert SUBAGENT_CAP_FLAG in plan, (
            f"every plan must pass the subagent cap: {plan!r}"
        )
    assert (
        "Pass `--subagent-cap` exactly as written: the shell expands it to "
        "the cap Claude Code enforces, or to nothing when it is unset."
    ) in _conduct()


def test_research_topic_leaves_concurrency_to_outstanding() -> None:
    prose = _research_topic_prose()
    assert "config get research.topic.concurrency" not in prose, (
        "the skill must not read concurrency; outstanding resolves it"
    )
    knobs = _section(prose, "knobs bound the research", "## Shared Preamble")
    assert (
        "Breadth's and depth's configured values, each resolved "
        "`personal > team > built-in default`, are below"
    ) in knobs
    assert (
        "`outstanding` resolves concurrency itself and bounds it by the "
        "subagent cap"
    ) in knobs
    assert (
        "(`--depth` or `--concurrency` on `outline`, `--breadth` on `conduct`)"
    ) in knobs, "the misplaced-flag rule must name --concurrency on outline"


def _rust_text(path: Path) -> str:
    return re.sub(r"\\\n\s*", "", _read(path))


def test_the_knob_rule_reads_alike_in_prose_and_in_the_cli() -> None:
    cli = _rust_text(PLAN_WORDING)
    for fragment in (
        "must be a positive integer, got '",
        "' — clamping to 1",
    ):
        assert fragment in _research_topic_prose()
        assert fragment in cli
```

`_rust_text` joins `\`-continued string literals, so wrapping a format
string at 80 columns cannot fail the test. It matches the expected strings
in `every_concurrency_diagnostic_names_its_source`, which that Rust test
ties to the rendered output, so a production string that drifts fails the
Rust test first.

**File**: `skills/research/research-topic/SKILL.md` (green)
**Changes**:

- **Knob block (`:59-67`):** keep the definition of all three knobs. The
  lead-in becomes "Breadth's and depth's configured values, each resolved
  `personal > team > built-in default`, are below:". Inject only breadth
  and depth, and add:

  > `outstanding` resolves concurrency itself and bounds it by the
  > subagent cap: an `--concurrency N` flag on `conduct`, else the
  > configured value, validated and then clamped to
  > `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` or 20. `conduct` passes the flag
  > and the cap through and never resolves concurrency itself.

  The "flag > resolved value above" rule and its empty-value, valid and
  out-of-range sub-rules now name breadth and depth only. The
  misplaced-flag sub-rule is unchanged and still names `--concurrency` on
  `outline`. "There is no upper bound; breadth is the per-round cost
  guard." stays, because it now governs only breadth and depth.
- **Step 1 (`:234-236`):** "Resolve depth per the knob-resolution rule
  above, reading any `--depth N` flag on the invocation. Set
  `{concurrency flag}` to `--concurrency 'N'` when the invocation carries
  `--concurrency N`, and to nothing otherwise. Pass `--subagent-cap`
  exactly as written: the shell expands it to the cap Claude Code
  enforces, or to nothing when it is unset."
- **Step 2 (`:244-254`):** `--limit {concurrency}` becomes
  `{concurrency flag} --subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"`
  in both commands. "report any warning that a stored
  ledger was replaced" becomes "report the first plan's `warnings` as soon
  as it returns — a replaced ledger or a clamped concurrency".
- **Step 7.2 (`:396-401`):** "without `--limit` or any run flag" becomes
  "with `{concurrency flag}`, the subagent cap and no run flag", and the
  command gains
  `{concurrency flag} --subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"`
  after `--depth {depth}`.

#### 6. Configure docs (red, then green)

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**: Add the configure cap pin (red).

```python
def test_configure_documents_the_subagent_cap_clamp() -> None:
    research = _configure_research_section()
    for sentence in (
        "`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`",
        "A `concurrency` above the subagent cap is clamped to the cap",
        "otherwise 20",
        "An empty value is refused rather than clamped.",
        "To spawn more than 20 at once, raise both "
        "`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` and `concurrency`.",
    ):
        assert sentence in research, f"configure must state {sentence!r}"
```

**File**: `skills/config/configure/SKILL.md` (green)
**Changes**: Rewrite the resolution paragraph after the table, and add a
short concurrency paragraph after it.

> Resolution order for each knob is **flag > personal > team > built-in
> default** ... A resolved value that is not an integer of 1 or more is
> clamped to 1 with a warning naming the invalid value. An empty value is
> refused rather than clamped. Breadth and depth have no upper bound —
> breadth is the per-round cost guard.
>
> **Concurrency and the subagent cap.** A `concurrency` above the
> subagent cap is clamped to the cap: the cap is
> `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` when set to a positive integer,
> otherwise 20, Claude Code's own limit. A flag or configured value above
> the cap warns; the built-in default clamps silently.
>
> - Set the variable in the shell that launches Claude Code, or under
>   `env` in a Claude Code settings file. `conduct` reads it through
>   Claude Code's Bash tool, so it must reach that tool's environment.
> - The variable accepts surrounding whitespace and a leading `+`, as
>   Claude Code does; `concurrency` accepts only plain digits.
> - To spawn more than 20 at once, raise both
>   `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` and `concurrency`.
> - The clamp applies on every Claude Code version and mode, including
>   those that enforce no cap.

#### 7. Reference docs and changelog

**File**: `docs-site/src/content/docs/research.md:470-513`
**Changes**:
- The unconditional paragraph that lists the JSON's top-level fields
  (`items`, `pairs`, `skipped`, `warnings`) gains `concurrency`, "the
  most spawns this plan offers, after the clamp", and `remaining`, "the
  spawns held back by the concurrency", since every plan carries both.
  Its lead-in becomes "It prints JSON with four arrays ... and two
  counts:".
- The `--depth N` field list drops `remaining` and keeps `unfinished`.
- Add a paragraph on how `--concurrency N` resolves: it falls back to
  `research.topic.concurrency`, is validated, is clamped to the cap, and
  is reported as `concurrency`. The cap is `--subagent-cap V`, read as
  Claude Code reads `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, or 20 when the
  flag is absent or holds a value Claude Code would ignore. `outstanding`
  reads no environment variable for it: `conduct` passes
  `--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"`, and a hand run
  passes the same to match. Every plan is bounded, a plain query
  included. The reported `concurrency` is the clamped value, so it can
  differ from the configured knob; the reason appears in `warnings` on
  `--start` only.
- The exit-code table replaces the `E_TOPIC_RESEARCH_LIMIT` row with
  `` `E_TOPIC_RESEARCH_CONCURRENCY` | `--concurrency` or `research.topic.concurrency` is empty | pass a positive integer, or set the config value to one or remove it ``.

**File**: `CHANGELOG.md`
**Changes**: Amend the unreleased entries in place.
- **`:124`:** `--limit` becomes `--concurrency`. The entry gains:
  `--concurrency` falls back to `research.topic.concurrency`, is clamped
  to the subagent cap given by `--subagent-cap` (20 without it), and is
  reported as `concurrency`, and an empty value exits 1 with
  `E_TOPIC_RESEARCH_CONCURRENCY`.
- **`:139-142`:** the concurrency entry gains the clamp. `conduct` never
  spawns more than the subagent cap, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`
  or 20; a configured or flag value above it is clamped with a warning.

### Success Criteria:

#### Automated Verification:

- [x] Domain and window tests pass: `cargo nextest run -p research` (from `cli/`)
- [x] CLI unit and integration tests pass: `cargo nextest run -p accelerator-research` (from `cli/`)
- [x] Structural tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] The skill permission lint passes: `mise run check`
- [x] The public API snapshot matches: `mise run public-api:check`
- [x] No `--limit` invocation or code remains: `rg -n -- '--limit \{concurrency\}|E_TOPIC_RESEARCH_LIMIT' skills docs-site/src/content/docs/research.md cli/research-cli` prints nothing
- [x] The CLI names no harness variable: `mise run lint:claude-coupling:check` exits 0
- [x] `mise run` exits 0, including `docs:check`

#### Manual Verification:

- [x] Launch with `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` exported in the shell and no override; the first plan reports `concurrency` 8 and no warning
- [x] Check that no `outstanding` command raises a permission prompt. Run; the check failed: under `--permission-mode default`, Claude Code blocks the expanded `--subagent-cap`, so Phase 4 replaces the flag and the remaining cap-source runs move there

---

## Phase 4: The CLI Reads the Subagent Cap

### Overview

Read the cap from the environment in one `research-cli` module, and drop
`--subagent-cap` from `outstanding` and from every plan in the skill. The
cap is the first of `ACCELERATOR_SUBAGENT_CAP` and
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` that holds a value the harness would
honour, else 20. `ACCELERATOR_SUBAGENT_CAP` comes first because it is the
explicit, agent-neutral setting. A value the harness would ignore falls
through to the next variable, silently, as the harness itself ignores it.
The coupling guard admits `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` by name, in
its reader and that reader's integration test only.

### Changes Required:

#### 1. Precedence in the domain (red, then green)

**File**: `cli/research/tests/concurrency.rs`
**Changes**: Add `the_first_honoured_setting_is_the_cap`, table-driven:

| Settings, in order | Expected |
|---|---|
| `"30"`, `"8"` | 30 |
| `"abc"`, `"8"` | 8 |
| `"0"`, `" 8 "` | 8 |
| unset, `"8"` | 8 |
| `"abc"`, `"-5"` | 20 |
| unset, unset | 20 |

**File**: `cli/research/src/conduct/concurrency.rs`
**Changes**: Add the rule, and express `from_setting` through it.

```rust
impl SubagentCap {
    /// The first setting the harness would honour, else the default.
    ///
    /// Each setting is read as the harness's JavaScript parser reads its
    /// own: trimmed as `String.prototype.trim` trims, an optionally signed
    /// run of ASCII digits, at least 1.
    #[must_use]
    pub fn first_honoured<'a>(
        settings: impl IntoIterator<Item = Option<&'a str>>,
    ) -> Self {
        settings
            .into_iter()
            .flatten()
            .find_map(harness_slots)
            .map_or(Self::HARNESS_DEFAULT, Self)
    }

    #[must_use]
    pub fn from_setting(setting: Option<&str>) -> Self {
        Self::first_honoured([setting])
    }
}
```

**File**: `cli/research/tests/fixtures/public-api.txt`
**Changes**: Regenerate; the diff adds only `first_honoured`.

#### 2. The reader (red, then green)

**File**: `cli/research-cli/src/subagent_cap.rs` (new), declared in
`main.rs`

```rust
//! The subagent cap, read from the environment `conduct` runs in.

use research::conduct::concurrency::SubagentCap;

const ACCELERATOR_CAP: &str = "ACCELERATOR_SUBAGENT_CAP";
const HARNESS_CAPS: [&str; 1] = ["CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"];

/// The first cap variable holding a value the harness would honour, the
/// accelerator's own before any harness's, else the harness default.
pub fn from_environment(
    lookup: impl Fn(&str) -> Option<String>,
) -> SubagentCap {
    let settings: Vec<Option<String>> = std::iter::once(ACCELERATOR_CAP)
        .chain(HARNESS_CAPS)
        .map(&lookup)
        .collect();
    SubagentCap::first_honoured(settings.iter().map(Option::as_deref))
}
```

`std::iter::once` stands in for a one-item array, which clippy's
`iter_on_single_items` rejects.

Its `tests` module drives `from_environment` with a lookup over a fixed
map: `the_accelerator_cap_wins_over_the_harness_cap` (30 and 8 → 30),
`an_accelerator_cap_the_harness_would_ignore_falls_through` (`abc` and 8 →
8), `the_harness_cap_alone_is_read` (8), and
`without_either_variable_the_cap_is_the_harness_default` (20).

#### 3. Drop the flag

**File**: `cli/research-cli/src/topic_command.rs`
**Changes**:
- `OutstandingFlags` loses `subagent_cap`, and `parse` its `subagent_cap`
  parameter.
- `resolve_concurrency` keeps its `cap` parameter.
- `the_subagent_cap_flag_bounds_the_configured_value` becomes
  `a_lowered_cap_bounds_the_configured_value`, resolving a configured `24`
  under `SubagentCap::from_setting(Some("8"))`.
- `without_a_subagent_cap_flag_the_cap_is_the_harness_default` is deleted;
  the reader's tests own that case.

**File**: `cli/research-cli/src/cli.rs`
**Changes**: Remove `subagent_cap`. The `concurrency` doc ends "and is
clamped to the subagent cap, read from `ACCELERATOR_SUBAGENT_CAP` or the
harness's own setting".

**File**: `cli/research-cli/src/main.rs`
**Changes**: `outstanding` passes
`subagent_cap::from_environment(|name| std::env::var(name).ok())` to
`resolve_concurrency`.

#### 4. Integration tests over the environment (red, then green)

**File**: `cli/research-cli/tests/topic_outstanding.rs`
**Changes**:
- Every `Command` the harness builds comes from one `command()` helper
  that removes both variables first, so a cap in the developer's
  environment, or in the Claude Code session running `mise run`, never
  reaches a test.
- `Project` gains `outstanding_in(args, cap: &[(&str, &str)])` and
  `plan_in`, which set the given variables on top.
- Every Phase 3 test passing `--subagent-cap V` sets
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=V` instead, keeping its name and
  expectations. The empty-value case sets the variable to `""`.
- New: `the_accelerator_cap_wins_over_the_harness_cap` (config 24,
  `ACCELERATOR_SUBAGENT_CAP=30`, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8`,
  `--start` → 24, no warnings).
- New: `an_accelerator_cap_the_harness_would_ignore_falls_through`
  (`ACCELERATOR_SUBAGENT_CAP=abc`, `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8`,
  `--start` → 8, no warnings).
- New: `subagent_cap_is_not_an_outstanding_flag` (`--subagent-cap 8` → exit
  2, stderr names `--subagent-cap` as unexpected).

#### 5. Coupling guard admits the cap by name (red, then green)

**File**: `tests/unit/tasks/test_claude_coupling.py`
**Changes**: Add a `TestOptionalHarnessVariables` class, each probe in
`tmp_path` like its siblings:
- `test_permits_the_cap_in_its_reader_and_its_test`;
- `test_flags_the_cap_anywhere_else_under_cli`;
- `test_flags_the_cap_in_bin_accelerator`;
- `test_flags_another_claude_name_in_the_reader`;
- `test_flags_a_line_naming_the_cap_beside_a_banned_name`;
- `test_the_admitted_set_is_exactly_the_subagent_cap`, pinning the mapping,
  so admitting a name is a reviewed change to this test;
- `test_a_missing_reader_raises`, as `test_a_missing_named_file_raises`
  does for `_FILES`.

The probe tree helper `_tree` writes every admitted file beside the
`_FILES` entries. Without them, every existing probe raises for a missing
reader once the guard checks the admitted files exist.

**File**: `tasks/lint/claude_coupling.py`
**Changes**: Add the mapping, and flag a `CLAUDE_*` match unless the
mapping names it and lists the file. A missing listed file raises, as a
missing `_FILES` entry does.

```python
_OPTIONAL_HARNESS_VARIABLES = {
    "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS": (
        "cli/research-cli/src/subagent_cap.rs",
        "cli/research-cli/tests/topic_outstanding.rs",
    ),
}
```

A line is flagged unless every `CLAUDE_*` name on it is admitted for that
file, so a banned name beside the cap still fails. The docstring keeps the
invariant and states the admission rule: a name
is admitted only if the CLI works without it, with the default stated
where it is read; admission is by name and file, never by file alone.

#### 6. ADR-0071

**File**: `meta/decisions/ADR-0071-optional-harness-variables-in-the-cli.md`
**Changes**: Draft from the ADR template, status `proposed`;
`/accelerator:create-adr` is interactive, so it was written directly:
- **Context:** the guard's origin in `CLAUDE_PLUGIN_ROOT`, and the
  permission checker blocking a variable expanded in a Bash command.
- **Decision:** the CLI may read an optional harness variable that has a
  stated default, admitted by name and confined to its reader. An
  `ACCELERATOR_` variable for the same setting takes precedence.
- **Consequences:** the guard stays a textual rule with a pinned admission
  set; another agent's variable needs the same admission; tests must clear
  admitted variables.

#### 7. Skill and structural tests (red, then green)

**File**: `tests/unit/tasks/test_research_structure.py`
**Changes**:
- `PLAN_AT_DEPTH` becomes `f"--depth {{depth}} {CONCURRENCY_FLAG}"`, and
  `SUBAGENT_CAP_FLAG` goes.
- `test_every_plan_passes_the_shell_expanded_subagent_cap` becomes
  `test_no_plan_expands_a_shell_variable`. It asserts that no `outstanding`
  line contains `--subagent-cap`, `$CLAUDE_CODE` or `${CLAUDE_CODE`, and
  that the "Pass `--subagent-cap` exactly as written" sentence is gone.
- The knob test asserts "clamped to `ACCELERATOR_SUBAGENT_CAP`, else
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, else 20" and "`conduct` passes the
  flag through and never resolves concurrency itself".

**File**: `skills/research/research-topic/SKILL.md`
**Changes**:
- The knob block reads: "`outstanding` resolves concurrency itself and
  bounds it by the subagent cap: an `--concurrency N` flag on `conduct`,
  else the configured value, validated and then clamped to
  `ACCELERATOR_SUBAGENT_CAP`, else `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`,
  else 20. `conduct` passes the flag through and never resolves concurrency
  itself."
- Step 1 drops the `--subagent-cap` sentence.
- All three plan commands drop
  `--subagent-cap "$CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS"`.
- The tick plan reads "with `{concurrency flag}` and no run flag".

#### 8. Docs and changelog

**File**: `tests/unit/tasks/test_research_structure.py`,
`skills/config/configure/SKILL.md`
**Changes**: The configure paragraph names the cap as
"`ACCELERATOR_SUBAGENT_CAP`, else `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`,
each when set to a value Claude Code would accept, otherwise 20". It gains
the bullet "Set `ACCELERATOR_SUBAGENT_CAP` to pin a cap no higher than
Claude Code's; above it, Claude Code refuses the extra spawns." The
settings bullet says "Set either variable", and the parsing bullet "Each
variable accepts". Its "To spawn more than 20 at once" bullet is
unchanged. The configure test pins both new sentences.

**File**: `docs-site/src/content/docs/research.md`
**Changes**: The cap paragraph drops `--subagent-cap`. The cap is read from
`ACCELERATOR_SUBAGENT_CAP`, then `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`,
each read as Claude Code reads its own setting, else 20, so a hand run in
the same shell sees the cap `conduct` does.

**File**: `CHANGELOG.md`
**Changes**: "the subagent cap given by `--subagent-cap` (20 without it)"
becomes "the subagent cap, read from `ACCELERATOR_SUBAGENT_CAP` or
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` (20 without either)". The
concurrency entry's cap becomes "`ACCELERATOR_SUBAGENT_CAP`, else
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`, else 20".

### Success Criteria:

#### Automated Verification:

- [x] Domain tests pass: `cargo nextest run -p research --test concurrency` (from `cli/`)
- [x] CLI tests pass: `cargo nextest run -p accelerator-research` (from `cli/`)
- [x] Guard tests pass: `uv run pytest tests/unit/tasks/test_claude_coupling.py`
- [x] Structural tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] No plan or CLI flag names the cap transport: `rg -n -- '--subagent-cap' skills cli/research-cli/src docs-site/src/content/docs/research.md CHANGELOG.md` prints nothing
- [x] The guard admits only the reader: `rg -ln CLAUDE_ cli` prints exactly `cli/research-cli/src/subagent_cap.rs` and `cli/research-cli/tests/topic_outstanding.rs`
- [x] `mise run public-api:check` exits 0
- [x] `mise run` exits 0

#### Manual Verification:

Run each in a fresh headless session, `claude -p --permission-mode
default --allowedTools Skill --plugin-dir <repo>`, with
`ACCELERATOR_RESEARCH_BIN` pointing at the local build, against a copy of
the `topic-research-quarantine-set` fixture. Each run stops after the first
plan. Set each cap source alone.

- [x] With `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` exported, the first plan reports `concurrency` 8, no warning and no permission denial
- [x] With `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS: "8"` under `env` in the project `.claude/settings.json`, then `.claude/settings.local.json`, then the user `~/.claude/settings.json`, each first plan reports `concurrency` 8. If a source does not reach the CLI, list only the verified sources in the configure docs
- [x] With `ACCELERATOR_SUBAGENT_CAP=4` and `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` exported, the first plan reports `concurrency` 4
- [x] With neither set, `conduct <slug> --concurrency 50` reports `concurrency` 20 and the clamp warning naming `--concurrency`, 50 and 20 before spawning
- [x] No run records a permission denial for an `accelerator` command. Each run's only denial was a compound `cd … && ls`/`grep` the model composed to inspect the set, not a command the skill prescribes

---

## Phase 5: Capacity-Refusal Retry Rounds

### Overview

Teach step 4 to re-issue spawns the harness capacity-refuses, in rounds,
before step 5's check and the step 6 re-plan. Teach step 5 that a capacity
refusal is not one of its no-file reasons, and add the exhaustion reason
to the failure-reason table.

### Changes Required:

#### 1. Structural pins (red)

**File**: `tests/unit/tasks/test_research_structure.py`

```python
CAPACITY_REASON = (
    "refused by the concurrent subagent limit — other subagents held the "
    "slots, or the harness cap is below the resolved concurrency"
)


def _spawn_step() -> str:
    return _section(
        _research_topic_prose(), "**Spawn the batch**", "**Check each spawn**"
    )


def _check_step() -> str:
    return _section(
        _research_topic_prose(), "**Check each spawn**", "**Loop.**"
    )


def test_the_spawn_step_re_issues_capacity_refused_spawns_in_rounds() -> None:
    spawn = _spawn_step()
    for sentence in (
        "Concurrent subagent limit reached",
        "a refusal citing the concurrent subagent limit",
        'This retry deliberately overrides the harness\'s "Do not retry".',
        "before step 5 and before any re-plan",
        "When the batch message capacity-refuses any spawn:",
        "The first round always runs.",
        "A later round runs only when the latest round had at least one "
        "spawn accepted.",
        "Once a round has every spawn capacity-refused, stop retrying.",
        "it is already recorded, so it is not recorded again",
        "The final summary states the `concurrency` the plans reported",
        CAPACITY_REASON,
    ):
        assert sentence in spawn, f"the spawn step must state {sentence!r}"


def test_the_check_step_leaves_capacity_refusals_to_the_spawn_step() -> None:
    assert (
        "A capacity refusal is not one of these; the spawn step handles it."
        in _check_step()
    )


def _capacity_row() -> str:
    rows = [
        line
        for line in _read(RESEARCH_TOPIC).splitlines()
        if line.lstrip().startswith(f'| "{CAPACITY_REASON}"')
    ]
    assert len(rows) == 1, "research-topic must carry one capacity row"
    return rows[0]


def test_the_capacity_row_names_re_running_conduct() -> None:
    row = _capacity_row()
    assert "re-run `conduct`" in row
    assert "`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`" in row
    assert "any `ACCELERATOR_SUBAGENT_CAP` is no higher" in row
    assert "below the `concurrency` the summary reports" in row
    assert "a smaller `--concurrency` or `research.topic.concurrency`" in row
```

#### 2. Spawn step (green)

**File**: `skills/research/research-topic/SKILL.md`, step 4
**Changes**: After the untrusted-summary paragraph that closes step 4, add:

> **Capacity refusals.** A Task call the harness refuses with a refusal
> citing the concurrent subagent limit (today "Concurrent subagent limit
> reached") never started: it is a capacity refusal, not a spawn that
> failed. Re-issue capacity-refused spawns, unchanged, in
> rounds within this step, before step 5 and before any re-plan. This
> retry deliberately overrides the harness's "Do not retry". A round is
> one follow-up message of re-issued spawns; the batch message is not a
> round. Like the batch message, a round returns once its accepted spawns
> have finished.
>
> When the batch message capacity-refuses any spawn:
>
> - The first round always runs. It carries every spawn the batch message
>   capacity-refused, with no added delay.
> - Each later round re-issues the spawns the latest round
>   capacity-refused. A later round runs only when the latest round had at
>   least one spawn accepted.
> - Once a round has every spawn capacity-refused, stop retrying. Record
>   each spawn still refused as failed, by pair and lineage, with the
>   reason "refused by the concurrent subagent limit — other subagents
>   held the slots, or the harness cap is below the resolved concurrency".
>   The step 6 re-plan lists it as `unfinished`; it is already recorded,
>   so it is not recorded again. The final summary states the
>   `concurrency` the plans reported beside any such failure.

#### 3. Check step (green)

**File**: `skills/research/research-topic/SKILL.md`, step 5
**Changes**: Append to the no-file bullet: "A capacity refusal is not one
of these; the spawn step handles it."

#### 4. Reason table (green)

**File**: `skills/research/research-topic/SKILL.md`, the table after step 7
**Changes**: Add a row after "a refused note, or "wrote no note"":

```markdown
| "refused by the concurrent subagent limit — other subagents held the slots, or the harness cap is below the resolved concurrency" | re-run `conduct` once other subagents have finished, and the new run offers the spawn again; if it recurs with no other subagents running, make sure `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` is set where both Claude Code and its Bash tool see it, that any `ACCELERATOR_SUBAGENT_CAP` is no higher than Claude Code's cap, or lower the concurrency below the `concurrency` the summary reports, with a smaller `--concurrency` or `research.topic.concurrency` |
```

#### 5. Changelog

**File**: `CHANGELOG.md`
**Changes**: Extend the concurrency entry: `conduct` re-issues spawns
Claude Code refuses for capacity (verified on Claude Code 2.1.296), and
records a spawn still refused once retrying stops, for a later run to
retry.

### Success Criteria:

#### Automated Verification:

- [ ] Structural tests pass: `uv run pytest tests/unit/tasks/test_research_structure.py`
- [ ] `mise run check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification:

These are the work item's manual runs, each captured from the session
transcript:
- the `concurrency` each plan reports;
- the spawns each plan offers;
- where each round sits relative to step 5 and the re-plan;
- every warning;
- the harness's refusal results;
- the final summary's failures;
- the re-plan's `unfinished` list.

**Holder setup.** A holder is a background `general-purpose` subagent told
to poll for a named stop file without the shell `sleep` command. It runs
a foreground
`python3 -c "import os,time\nwhile not os.path.exists('<stop>'): time.sleep(2)"`
with a 600000 ms Bash timeout, and repeats it until the file exists.
Before each hold run, a probe spawn validates the fixture and must finish
before `conduct` starts:
- the partial-hold probe is accepted;
- the full-hold probe is capacity-refused.

A run whose probe gives the other result is invalid rather than failed.
So is a run whose first batch offers fewer spawns than it needs, and a
hold run whose first plan reports a `remaining` other than 0. If the
issue #140 brief is no longer retrievable, the full-load run uses any brief
whose batches offered more than 20 spawns before the fix.

- [ ] **Full load:** with default config and no holders, run `conduct <slug> --depth 3`. At least one batch offers exactly 20 spawns, all accepted by the batch message. Every plan reports `concurrency` 20, and no capacity refusal appears.
- [ ] **Partial hold:** launch with `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=4`, start 3 holders, then run `conduct <slug> --depth 2` on a brief with exactly 4 level-1 spawns, so the first plan offers 4 with `remaining` 0. The batch message has 1 spawn accepted and 3 refused, and rounds then accept 1 spawn each until none is left. The summary has no capacity failure, and the re-plan lists no re-issued spawn as `unfinished`.
- [ ] **Full hold:** launch with `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=4`, start 4 holders, then run `conduct <slug> --depth 2` on the same brief. The batch message and exactly one round are fully refused. The summary lists each spawn once with the capacity reason. After the stop file is created, a fresh `conduct` run offers those spawns again.

---

## Testing Strategy

### Unit Tests:

- **Cap parser:** `SubagentCap::from_setting` over every case the work item
  names, plus `-0`, whitespace-only, a bare sign, U+FEFF, U+0085, a
  non-ASCII digit, a long zero-padded value and u32 overflow.
- **Request resolution:** `RequestedConcurrency::resolve` over:
  - each provenance at, below and above the cap;
  - validation before the clamp, from the flag and from config;
  - the empty refusal, carrying its provenance;
  - an over-large `given` kept verbatim in the warning.
- **Window bound:** `Concurrency::admits` and the window and ledger unit
  tests over a `Concurrency`.
- **Wording:** every concurrency diagnostic names its source
  (`plan_wording.rs`).
- **Precedence:** a flag is resolved without reading config, and config
  is the fallback (`topic_command.rs`). The cap is the first honoured of
  `ACCELERATOR_SUBAGENT_CAP` and `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`
  (`concurrency.rs` tests, `subagent_cap.rs`).
- **Coupling guard:** the cap is admitted by name in its reader and its
  test only, and the admitted set is pinned (`test_claude_coupling.py`).
- **Default at the cap:** the catalogue default equals
  `SubagentCap::HARNESS_DEFAULT` (`context.rs`).

### Integration Tests:

- **`outstanding` binary end to end:**
  - both cap variables cleared on every call, then set explicitly, with
    team and personal config files;
  - exact `warnings`, stderr and `concurrency`, with clean stderr where no
    warning is expected;
  - the offered spawns and `remaining` under each clamp;
  - warnings on `--start` only, ahead of a replaced-ledger notice;
  - the empty-value exit with no ledger written, and a flag standing in
    for an empty configured value;
  - `--limit` and `--subagent-cap` rejected.
- **Structural tests:**
  - the skill's flag pass-through, including the tick plan;
  - the knob block, and knob-rule wording parity with the CLI;
  - the spawn step's ordering, stop and single-recording rules;
  - the check step, the reason row, and the configure docs.

### Manual Testing Steps:

1. Phase 1: `accelerator config get research.topic.concurrency` prints `20`.
2. Phase 4: the cap-reaches-the-CLI runs, with no permission denial.
3. Phase 5: the full-load, partial-hold and full-hold runs.

## Performance Considerations

⏱️ A full batch drops from 24 to 20 spawns, so a `--depth 3` run has up to
20% more batches. A `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` above 20, with
a matching `research.topic.concurrency`, restores wider batches. Retry
rounds add one message per round. Each round waits on its accepted spawns,
so no busy loop is added.

## Migration Notes

The knob and the run flags are unreleased (`## [Unreleased]`), so the
`--limit` to `--concurrency` rename needs no deprecation path. Projects
that configured `research.topic.concurrency` above 20 for runs on a
pre-cap harness get a clamp warning. Raising
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` removes it. The 0295 validation's
advice to set `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=24` for arXiv step-17
runs is now required for a 24-wide load.

⚠️ The clamp to 20 applies whatever the harness version or mode. Claude
Code below v2.1.217 (the plugin floor is v2.1.144) and ultracode sessions
enforce no cap, yet still get batches of 20 and a clamp warning for a
higher configured value. Setting `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`
raises the CLI's cap even where the harness ignores it.

A plain `outstanding` query, with no run flag, was unbounded and read no
research config. It now offers at most the resolved concurrency, counts the
rest in `remaining`, and exits 1 on an empty `research.topic.concurrency`.
Ticking is unaffected, because `items[].complete` comes from the round
plan, not the window.

The capacity retry recognises the harness's refusal by meaning, with
"Concurrent subagent limit reached" as today's text, verified on Claude
Code 2.1.296. On a harness upgrade, recheck the refusal text and
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` parsing against the spawn step and
`a_cap_is_parsed_as_the_harness_parses_it`.

`--subagent-cap` existed only between Phase 3 and Phase 4, unreleased, so
its removal needs no deprecation path. `ACCELERATOR_SUBAGENT_CAP` is new and
optional.

The default of 20 and `SubagentCap::HARNESS_DEFAULT` encode one decision,
that the default sits at the harness cap.
`the_built_in_default_sits_at_the_harness_cap` ties them together, so a
change to the harness cap forces a decision about the default.

## References

- Original work item: `meta/work/0303-research-concurrency-exceeds-subagent-limit.md`
- Codebase research: `meta/research/codebase/2026-10-10-0303-research-concurrency-subagent-limit.md`
- Run ledger and `--limit`: `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`
- Knob rule: `meta/plans/2026-09-20-0282-tunable-depth-and-breadth.md`
- Coupling guard: `tasks/lint/claude_coupling.py`; the decision to admit
  optional harness variables: ADR-0071 (Phase 4)
- Models: `cli/research/src/topic/tree.rs:19-43` (`Depth`),
  `topic_command.rs:225-228` (`--start`-only notice)
