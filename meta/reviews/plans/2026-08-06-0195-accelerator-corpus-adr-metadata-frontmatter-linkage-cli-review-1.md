---
type: "plan-review"
id: "2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-review-1"
title: "Plan Review: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan"
date: "2026-08-06T09:35:21+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "compatibility", "safety", "usability"]
review_number: 1
review_pass: 2
tags: ["rust", "corpus", "cli", "adr", "frontmatter", "linkage", "sub-binary"]
last_updated: "2026-08-07T13:33:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan

**Verdict:** REVISE

The plan is disciplined about golden-baseline fidelity and reuse — it correctly identifies which domain logic already exists (`corpus::linkage`, `corpus_adapters::metadata`), mirrors the `vcs-cli` sub-binary precedent for scaffolding, and explicitly enumerates several genuinely obscure bash quirks it commits to preserving. Phase 4 (`frontmatter validate`), however, contains two real logic divergences from the bash validator it replaces — one of them a byte-for-byte parity break that a bash-era golden test already exists to catch — plus a cluster of design and test-coverage gaps that make it the highest-risk phase in the plan. Several smaller but consistent findings recur across lenses in the command layer's config-composition approach, warranting a revision pass before implementation begins.

### Cross-Cutting Themes

- **Phase 4 (`frontmatter validate`) is the plan's highest-risk area, flagged independently by five lenses** (flagged by: Correctness, Code Quality, Usability, Test Coverage, Safety) — Correctness found two real semantic divergences from bash (type resolution, short-circuit ordering); Code Quality and Usability independently flagged the same implicit whole-corpus-vs-file-list mode switch as a design smell; Test Coverage found the replacement test suite is narrower than the 49-case bash suite it retires; Safety flagged that the by-name-required CI gate being retired has no confirmed equivalent guarantee on the Rust side. Five lenses converging on one phase is a strong signal it needs the most attention before implementation.
- **`table_from_config` is under-designed** (flagged by: Standards, Architecture, Test Coverage) — Standards found it duplicates an existing sibling helper (`table_from_paths`) that already does the same job via a different key; Architecture found its proposed home (`corpus-adapters`) would add a `config` dependency to a crate that is deliberately config-agnostic today; Test Coverage found it has no described unit test of its own. All three findings point at the same ~10-line function.
- **Command-layer testability lags the `vcs-cli` precedent** (flagged by: Architecture, Code Quality) — both lenses independently compared the plan's command modules against `cli/vcs-cli/src/detect.rs`'s injected-port pattern (`ModeProbe`/`CheckoutProbe` + `StubProbe`) and found the new command functions compose concrete adapters inline, with no described unit-test seam for their branching logic — leaving black-box binary-spawn tests as the only described coverage for this layer.
- **Byte-for-byte parity has real gaps in the CLI/exit-code layer, not just the domain logic** (flagged by: Compatibility, Correctness) — clap's own usage-error exit code (2) conflicts with bash's exit code 1 for the ADR subcommands, and a regex-valid-but-`u32`-overflowing `--count` value has no defined behaviour, both of which sit outside the domain functions the plan otherwise characterizes carefully.

### Tradeoff Analysis

- **Slavish bash-shape mirroring vs idiomatic Rust CLI design**: The plan is explicit that CLI argument *shape* (as opposed to domain-level output) isn't required to match bash byte-for-byte (see "What We're NOT Doing"), yet in practice it mirrors bash argument shapes in places that cost discoverability — `linkage extract`'s bare optional positional for `source_type` (Usability) and `frontmatter validate`'s implicit mode-by-argument-shape (Code Quality, Usability) both read as accidental parity rather than deliberate choices. Recommendation: where the "What We're NOT Doing" exemption already grants freedom from bash's shape, take it — prefer named flags and explicit mode selection over positional/shape-inferred behaviour that happens to match bash's old interface.
- **Byte-for-byte parity vs Rust-appropriate error handling**: AC1 requires exit codes to be characterized byte-for-byte, but clap's idiomatic built-in argument validation exits with its own convention (2) rather than bash's (1) for equivalent usage errors. Fully honouring AC1 here means giving up some of clap's automatic validation in favour of hand-rolled checks (as the plan already does for `--count`), which is a real ergonomics cost for the implementer. Recommendation: decide this explicitly and consistently per-subcommand rather than discovering the conflict case-by-case during implementation.

### Findings

#### Major

- 🟡 **Correctness**: `validate_path`'s path-inferred type fallback diverges from bash's frontmatter-only type resolution
  **Location**: Phase 4, Section 2 (Adapter module)
  Bash's `validate_file` resolves the type used for `INVALID-TYPE` solely from the frontmatter `type:` field — never from the path — and an existing golden fixture (`boundary-untyped.md`) already asserts this. The plan's `validate_path` falls back to path-inferred type, which would silently validate a file missing its `type:` declaration instead of flagging `INVALID-TYPE`, defeating a real enforcement guarantee.

- 🟡 **Correctness**: `dangling_refs` orchestration risks losing bash's short-circuit-on-invalid-type invariant
  **Location**: Phase 4, Section 2/3 (`validate_corpus` orchestration)
  Bash returns immediately on `NO-FENCE`/`INVALID-TYPE`, never reaching the linkage-shape/dangling-ref checks. The plan describes `validate_path` and `dangling_refs` as run per file with no described gate linking them, risking a different violation set than the golden baseline for malformed files specifically.

- 🟡 **Compatibility**: Clap's default exit code (2) conflicts with bash's exit code (1) for ADR subcommand usage errors
  **Location**: Phase 1, Section 1 (`cli.rs`) / "What We're NOT Doing" scoping note
  `adr-next-number.sh`/`adr-read-status.sh` exit 1 for invalid invocations; clap's built-in required-argument/unrecognised-flag handling exits 2. The scoping note only exempts clap's `Usage:` text from parity, not the exit code, yet AC1 requires exit codes to be characterized byte-for-byte.

- 🔴 **Standards** / 🔴 **Architecture**: `table_from_config` duplicates an existing helper and is proposed for the wrong crate
  **Location**: Phase 3, Section 1 (Config → doc-type table adapter)
  `cli/corpus-adapters/src/doc_type.rs` already has `table_from_paths`, which builds the identical `(DocTypeKey, PathBuf)` table from a `HashMap` keyed by `config_path_key()`. The plan's new `table_from_config` reimplements the same mapping keyed by `linkage_type_name()` instead of delegating to `table_from_paths`. Separately, placing this function in `corpus-adapters` — a crate with zero `config` dependency today, consumed by `accelerator-visualiser` as well — couples every consumer to `config` for a capability only `accelerator-corpus` needs; the plan's own cited precedent (`WorkItemConfig`) composes config at the consuming binary, not inside the shared adapters crate.

- 🟡 **Code Quality** / 🟡 **Usability**: `frontmatter validate` silently switches modes based on argument shape
  **Location**: Phase 4, Section 3 (CLI + command layer)
  Whether whole-corpus mode (with referential-integrity checks) or file-list mode (without them) runs is inferred from whether `paths` has exactly one directory entry — nothing in the CLI signature communicates this, and the behaviour for multiple-directories or mixed file/directory arguments is unspecified, likely surfacing a raw OS error rather than a domain message.

- 🟡 **Architecture** / 🟡 **Code Quality**: Command-layer functions compose concrete adapters inline, with no injected-port testability seam
  **Location**: Phase 1, Section 3 (Command layer) and equivalents in Phases 3–4
  Unlike `cli/vcs-cli/src/detect.rs`'s `run<P: ModeProbe + CheckoutProbe>` (generic over an injectable port, unit-tested via `StubProbe`), the plan's command functions call `config_adapters::compose` and do filesystem I/O directly inside the same function that contains the response-shaping branches, leaving black-box binary-spawn golden tests as the only described coverage for this branching logic.

- 🟡 **Test Coverage**: No described test coverage for the config-composition wiring itself
  **Location**: Phase 1, Section 3 and Phase 3, Section 1
  The plan's own research flags config wiring as the riskiest genuinely-new surface, yet no described test exercises a non-default `.accelerator/config.md` value or the `LegacyPolicy::Reject` failure path — the riskiest logic would ship with the least direct test evidence.

- 🟡 **Test Coverage**: Frontmatter validator's test breadth shrinks sharply versus the 49-case bash suite it replaces
  **Location**: Phase 4, Section 4 (Tests)
  The replacement is described only as "at least one [unit test] per violation code" plus a handful of adapter/CLI tests, with no explicit statement of how multi-violation combinations, rule interactions, or per-type schema-row edge cases (covered by a presumably large fraction of the original 49 bash cases) are carried forward before the bash oracle is deleted.

#### Minor

- 🔵 **Architecture**: `paths.decisions` resolution is reimplemented in `corpus-cli` rather than extracted into a shared function the launcher could also use, risking silent behavioural drift between `accelerator config path decisions` and `accelerator corpus adr next-number`. *(Phase 1, Section 3)*
- 🔵 **Architecture** / 🔵 **Code Quality**: No shared composition-root helper (mirroring the launcher's `compose_stack`) is introduced for the three command modules that each independently call `config_adapters::compose`. *(Phases 1, 3, 4)*
- 🔵 **Code Quality**: The new `frontmatter_validation.rs` module folds a schema table, emission-rule constants, a 16-variant enum with byte-exact `Display`, a shape checker, and two entry points into one file — more surface than the `corpus::linkage` module it's modelled on. *(Phase 4, Section 1)*
- 🔵 **Test Coverage**: `table_from_config` has no standalone unit test, only indirect coverage via the Phase 3 CLI golden test. *(Phase 3, Section 1)*
- 🔵 **Test Coverage**: The `Malformed` frontmatter-parse variant (tagged YAML, fail-closed by design in `corpus_adapters::document::parse`) isn't mentioned in the domain, adapter, or test sections for `frontmatter validate`. *(Phase 4, Section 1)*
- 🔵 **Test Coverage**: The byte-significant em-dash (U+2014) violation separator isn't called out for an explicit character-level assertion, only golden-string comparison where a hand-transcribed hyphen typo could pass silently. *(Phase 4, Section 1 / Testing Strategy)*
- 🔵 **Test Coverage**: It's unstated whether the new `*_goldens.rs` CLI tests run unconditionally or risk being copy-pasted behind the `bash-parity` feature gate used by existing bash-comparison tests in this codebase. *(Phases 1–4, Tests sections)*
- 🔵 **Correctness** / 🔵 **Compatibility**: `--count`'s regex (`^[1-9][0-9]*$`) has no upper bound, but `next_numbers` takes `u32` — no described handling for a regex-valid, `u32`-overflowing value, risking an unhandled panic instead of the designed error path. *(Phase 1, Sections 2–3)*
- 🔵 **Correctness**: `read_status`'s quote-stripping description omits bash's order-of-operations quirk (prefix-strip → quote-strip → whitespace-trim, in that sequence), which can leave a stray trailing quote character bash would produce but the plan's prose wouldn't. *(Phase 1, Section 2)*
- 🔵 **Compatibility**: Exit-code mapping for `ConfigError` (Failed vs Refusal) is unspecified across all config-dependent subcommands, risking inconsistent exit codes for the same failure class bash treats uniformly as exit 1. *(Phase 1 §3, Phase 4 §3)*
- 🔵 **Standards**: The new `commands/` subdirectory layout has no precedent in `vcs-cli`'s flat per-verb module layout, which the plan otherwise cites as the shape to mirror, and the deviation isn't acknowledged. *(Phase 1, Section 1)*
- 🔵 **Standards**: `allowed-tools` wildcarding is inconsistent between Phase 1 (`adr *`, matching repo convention) and Phase 2 (`metadata derive`, exact match with no precedent elsewhere in `skills/`). *(Phase 1 §6 vs Phase 2 §3)*
- 🔵 **Safety**: The by-name-required CI gate being retired (`_REQUIRED_CONFIG_SUITES`) is replaced with an unconditional `cargo test`, but the plan doesn't confirm the new self-check test will actually run in the default `mise run check` invocation with no exclude/feature-gate — the exact silent-loss failure mode the retired mechanism existed to prevent. *(Phase 4, Removal / Tests)*
- 🔵 **Safety**: The referential-integrity index silently coalesces duplicate `(type, id)` keys with no violation raised for the collision, a gap inherited from bash but now sitting behind a check positioned as the permanent fail-closed guarantee. *(Phase 4, Section 2)*
- 🔵 **Usability**: `linkage extract`'s bare optional positional for `source_type` mirrors bash's shape even though "What We're NOT Doing" says CLI argument shape isn't constrained to bash parity; a named `--source-type` flag would be more discoverable via `--help`. *(Phase 3, Section 2)*
- 🔵 **Usability**: The illustrative `cli.rs` snippets across all phases omit the doc-comment convention `vcs-cli` uses to populate `--help` text. *(Phases 1, 3, 4)*
- 🔵 **Usability**: `count`'s hand-validated raw-`String` pattern (a deliberate, justified bash-parity exception) isn't marked as such, risking being copied for future arguments with no parity requirement. *(Phase 1, Section 1)*

#### Suggestions

- 🔵 **Safety**: File-list mode's silent skip of referential-integrity checking has no caller-facing warning; consider a stderr note or documentation callout. *(Phase 4, Command layer / Phase 5 docs)*
- 🔵 **Safety**: No test cases are enumerated for missing/malformed `.accelerator/config.md` during config composition, across any config-dependent subcommand. *(Phases 1, 3, 4)*
- 🔵 **Code Quality**: The "guard against octal misparse" note for `next_numbers` describes a bash-specific hazard (`$((...))` treating leading zeros as octal) that doesn't apply to Rust's `str::parse::<u32>()` — worth flagging explicitly so it isn't miscopied into unnecessary defensive code. *(Phase 1, Section 2)*
- 🔵 **Test Coverage**: `read_status`'s quote-unwrapping cases and a `count = 0` domain-boundary case aren't called out in the listed unit tests, even though the CLI layer currently guards against the latter. *(Phase 1, Section 4)*
- 🔵 **Compatibility**: `SystemClock`'s existing, documented hard-failure on unresolvable UTC offset (vs bash's silent degrade) is a known, pre-existing divergence — worth a one-line note in Phase 2's success criteria so it isn't mistaken for an oversight. *(Phase 2)*

### Strengths

- ✅ Reuses already-built, differentially-parity-tested domain logic (`corpus::linkage::parse_document`, `corpus_adapters::metadata::{derive_at,render}`) wherever it exists rather than re-deriving it.
- ✅ Explicitly enumerates and commits to preserving numerous genuinely obscure bash behaviours (ADR-number minimum-width formatting, octal-safe parsing, fence early-break, empty-status-still-succeeds, double-quote-only linkage shape, the hardcoded 4-digit bare-number scan) rather than silently "fixing" them mid-port.
- ✅ Correctly splits pure per-file rule evaluation from I/O-heavy whole-corpus walk/index building for the genuinely new frontmatter-validation logic, matching the established `corpus`/`corpus-adapters` hexagonal boundary.
- ✅ TDD-first structure throughout: tests written before handlers, golden-baseline output captured before deleting the bash oracle, with a smart transient-parity-test pattern (keep the differential test just long enough to confirm agreement, then delete it alongside its oracle).
- ✅ Phases are independently mergeable, with floor/registry edits scoped precisely to what each phase's own bash removal touches — good evolutionary discipline for a multi-week migration.
- ✅ All five new subcommands are read-only against the corpus — no new destructive-write path is introduced.
- ✅ The scaffolding, dispatch shape, and error-mapping convention closely and accurately mirror the already-shipped `vcs-cli` sub-binary.
- ✅ Phase 2 proactively closes a pre-existing `allowed-tools` gap (several skills invoked the bash metadata helper via prose with no matching grant) as part of the migration.
- ✅ Sensible zero-config defaults are preserved throughout (`adr next-number`/`metadata derive` both work with no arguments), and the `--count` error message is specific and actionable.
- ✅ Dependency additions are entirely workspace-inherited with no new version pinning or drift risk.

### Recommended Changes

1. **Fix `validate_path`'s type resolution to match bash exactly** (addresses: `validate_path`'s path-inferred type fallback diverges from bash). Resolve type strictly from frontmatter for the structural check; reserve path-inference for the `Index`-building step only, as the plan already does correctly there.

2. **Thread short-circuit state from `validate_path` into `validate_corpus`'s orchestration** (addresses: `dangling_refs` orchestration risks losing bash's short-circuit invariant). Skip `dangling_refs`/linkage-shape checks whenever `NO-FENCE`/`INVALID-TYPE` already fired for a file, matching bash's single-pass early return.

3. **Decide and state exit-code parity scope for clap-native usage errors** (addresses: clap exit-code conflict, config-composition exit-code mapping). Either hand-validate `adr read-status`'s `file` argument (as already planned for `--count`) to preserve exit 1, or explicitly scope this out of AC1 with justification; state the `ConfigError` → `Failed`/`Refusal` mapping per config-dependent command.

4. **Delegate `table_from_config` to the existing `table_from_paths` helper, and move it to `corpus-cli`** (addresses: table_from_config duplication and wrong-crate placement). Build a `HashMap<String, PathBuf>` keyed by `path_key` from `doc_type_dirs`'s output and pass it to `table_from_paths`; place the composition at `corpus-cli`'s own composition root, not inside `corpus-adapters`.

5. **Make `frontmatter validate`'s mode selection explicit** (addresses: silent mode switching by argument shape). Either add a `--corpus <ROOT>` flag (or a distinct verb) for whole-corpus/referential-integrity mode, reserving bare `paths` for file-list mode, and define the multi-directory/mixed-argument case explicitly with a test.

6. **Add a config-composition test to at least one command per config-dependent phase** (addresses: no coverage for the riskiest new wiring). Exercise a non-default `paths.decisions`/doc-type-dir value and the `LegacyPolicy::Reject` path.

7. **State how the frontmatter-validate test suite covers multi-violation and per-type-schema-row cases** (addresses: test breadth shrinkage vs the 49-case bash suite). Either transliterate a representative sample beyond one-per-violation-code before the bash oracle is deleted, or add deliberately multi-violation fixtures to the adapter/CLI test tier.

8. **Reconcile the command-layer testability gap with the `vcs-cli` precedent, at least where branching logic is non-trivial** (addresses: no injected-port seam for command modules). Extract pure decision logic (e.g. "given this directory listing, what should be printed") into functions taking already-resolved inputs, or explicitly note where the simpler inline approach is an accepted per-command tradeoff.

## Per-Lens Results

### Architecture

**Summary**: The plan cleanly reuses existing, already-parity-tested domain logic (`corpus::linkage`, `corpus_adapters::metadata`) and maintains a correct domain/adapter split for the genuinely new `frontmatter_validation` logic. Its most significant weakness is a quiet departure from the `vcs-cli` precedent and ADR-0053's composition-root principle: command-layer functions compose concrete adapters inline rather than being generic over injected ports, and `table_from_config` is proposed to live inside the shared, currently config-agnostic `corpus-adapters` crate rather than at the consuming binary's own composition root.

**Strengths**:
- Reuses already-built, differentially-parity-tested domain logic wherever it exists.
- Correct domain/adapter hexagonal split for the new frontmatter-validation logic.
- Phases independently mergeable with scoped floor/registry edits.
- Deliberately preserves documented bash quirks as conscious decisions.

**Findings**:
- 🟡 Major (high confidence): Command-layer functions compose concrete adapters inline instead of taking injected ports (Phase 1 §3).
- 🟡 Major (high confidence): `table_from_config` proposed for `corpus-adapters`, adding a new config dependency to a shared, currently config-agnostic crate (Phase 3 §1).
- 🔵 Minor (medium confidence): `paths.decisions` resolution logic reimplemented rather than shared with the launcher's equivalent (Phase 1 §3).
- 🔵 Suggestion (medium confidence): No shared composition-root helper across the three phases that each independently compose config (Phases 1, 3, 4).

### Code Quality

**Summary**: The plan is disciplined about reuse and follows the `vcs-cli` scaffolding precedent closely. The weakest points are in the command (I/O) layer: an implicitly dual-mode CLI argument for `frontmatter validate`, no visible shared composition-root helper, and a testability gap relative to `vcs-cli`'s injected-probe pattern. The new `frontmatter_validation` module is also large enough to warrant a second look at internal decomposition.

**Strengths**:
- New domain logic specified as pure, dependency-free functions mirroring `corpus::linkage`'s discipline.
- `table_from_config` reused unchanged between Phase 3 and Phase 4, avoiding duplication.
- Structured `Violation` enum with its own `Display`, consistent with existing domain-error precedent.
- Registration/scaffolding phase closely mirrors an existing, working sub-binary.

**Findings**:
- 🔴 Major (high confidence): `frontmatter validate` switches between two very different modes based on argument shape, not an explicit flag (Phase 4 command layer).
- 🟡 Major (medium confidence): Command-layer branching logic only exercised through binary-spawn golden tests, unlike `vcs-cli`'s injected-probe pattern (Phases 1, 3, 4).
- 🔵 Minor (medium confidence): Config composition likely duplicated across three command modules with no shared helper (Phases 1, 3, 4).
- 🔵 Minor (low confidence): New `frontmatter_validation` module folds several distinct concerns into one file (Phase 4 §1).
- 🔵 Suggestion (medium confidence): "Guard against octal misparse" note describes a bash-specific hazard that doesn't apply to Rust's own parsing (Phase 1 §2).

### Test Coverage

**Summary**: The plan follows a disciplined TDD structure with clear domain/adapter/CLI test layering, reuses existing differential bash-parity suites as transient oracles before retiring them, and explicitly enumerates several subtle bash quirks it commits to preserving with dedicated unit tests. The main gaps are proportional-to-risk: the genuinely new config-composition wiring has no described test coverage for non-default config values, and the 49-case bash frontmatter suite is replaced by a much smaller "one test per violation code" set without an explicit statement of how combination/interaction cases are covered.

**Strengths**:
- Consistent TDD structure per phase with a clear three-tier pyramid (domain, adapter, CLI black-box).
- Smart migration-safety device: `bash-parity`-gated differential tests kept just long enough to confirm agreement, then deleted alongside their oracle.
- Explicitly enumerates several non-obvious bash quirks committed to preservation with dedicated unit tests.
- Phase 4 adds an unconditional whole-corpus self-check as the permanent fail-closed migration-completion signal.

**Findings**:
- 🟡 Major (medium confidence): No described test coverage for the config-composition wiring itself (Phase 1 §3, Phase 3 §1).
- 🟡 Major (medium confidence): Frontmatter validator's test breadth shrinks sharply versus the 49-case bash suite it replaces (Phase 4 §4).
- 🔵 Minor (medium confidence): `table_from_config` has no standalone unit test (Phase 3 §1).
- 🔵 Minor (medium confidence): Malformed-frontmatter (tagged-YAML) path not mentioned in domain, adapter, or test sections (Phase 4 §1).
- 🔵 Minor (medium confidence): Byte-significant em-dash separator isn't called out for explicit character-level assertion (Phase 4 §1 / Testing Strategy).
- 🔵 Minor (low confidence): Unclear whether new `*_goldens.rs` CLI tests run unconditionally or are feature-gated like existing bash-comparison tests (Phases 1–4).
- 🔵 Suggestion (low confidence): Domain-level unit tests for `read_status`'s quote-unwrapping and `next_numbers`'s degenerate `count` not called out (Phase 1 §4).

### Correctness

**Summary**: The plan is unusually disciplined about golden-baseline fidelity, explicitly calling out several genuine bash quirks and committing to preserve rather than silently fix them. Close comparison against the actual bash sources surfaces two real logic divergences in the new `frontmatter validate` domain design that would silently weaken the type-conformance guarantee it replaces, plus two smaller unaddressed edge cases. No phase performs writes, so state-management/concurrency risk is minimal.

**Strengths**:
- Explicitly enumerates and commits to preserving several genuinely obscure bash behaviours.
- TDD-against-captured-goldens structure is right for catching this class of divergence.
- Every subcommand in scope is read-only, so state-management/atomicity concerns are largely not applicable.

**Findings**:
- 🔴 Major (high confidence): `validate_path`'s path-inferred type fallback diverges from bash's frontmatter-only type resolution for structural checks (Phase 4 §2) — an existing golden fixture (`boundary-untyped.md`) already documents the bash behaviour this would break.
- 🟡 Major (medium confidence): `dangling_refs` invoked unconditionally per file risks losing bash's short-circuit-on-invalid-type invariant (Phase 4 §2/3).
- 🔵 Minor (medium confidence): Quote-stripping description omits a bash order-of-operations quirk that leaves a stray trailing quote character (Phase 1 §2).
- 🔵 Minor (medium confidence): `--count` validation regex has no upper bound but the domain function takes `u32` (Phase 1 §3).

### Standards

**Summary**: The plan is unusually disciplined about mirroring the `vcs-cli` sub-binary precedent for scaffolding, dispatch, error mapping, and test naming, and correctly follows the codebase's untyped-frontmatter-navigation convention. Three concrete convention inconsistencies stand out: a new `commands/` module layout diverging from `vcs-cli`'s flat layout, inconsistent `allowed-tools` wildcarding between Phase 1 and Phase 2, and a new doc-type-table adapter function that reinvents a mapping key already used by an existing sibling helper.

**Strengths**:
- Binary scaffold, dispatch shape, and error mapping explicitly and accurately mirror the shipped `vcs-cli` precedent.
- Correctly follows the established untyped `Mapping`/`FrontmatterValue` navigation convention rather than introducing typed deserialization.
- `allowed-tools` rewrites correctly target existing grant patterns and Phase 2 proactively closes a pre-existing gap.

**Findings**:
- 🔴 Major (high confidence): New `table_from_config` duplicates an existing sibling helper's mapping (`table_from_paths`) instead of reusing it (Phase 3 §1).
- 🔵 Minor (medium confidence): `commands/` module layout diverges from the `vcs-cli` precedent the plan says it mirrors (Phase 1 §1).
- 🔵 Minor (medium confidence): Inconsistent `allowed-tools` wildcarding between Phase 1 and Phase 2 (Phase 1 §6 vs Phase 2 §3).

### Compatibility

**Summary**: The plan is unusually disciplined about output-contract stability — repeatedly calling out byte-for-byte preservation of TSV field order, the em-dash violation format, minimum-width zero-padding, and other quirky bash behaviours. The main compatibility risk it does not surface is that clap's own argument-parsing errors exit with code 2 while the bash scripts being replaced exit 1 for equivalent usage errors — a direct conflict with AC1, only partially and coincidentally resolved for `frontmatter validate`.

**Strengths**:
- Output formats consumers depend on are explicitly pinned to bash's exact byte-for-byte shape.
- Domain-level exit-code convention reused unchanged from the already-shipped `accelerator-vcs` sub-binary.
- Dependency additions entirely workspace-inherited.
- Preserves several deliberately-quirky bash behaviours a naive rewrite would silently "fix".
- `ACCELERATOR_CORPUS_BIN` override follows the existing `ACCELERATOR_VCS_BIN` naming convention.

**Findings**:
- 🔴 Major (high confidence): Clap's default usage-error exit code (2) conflicts with bash's own usage-error exit code (1) for the ADR subcommands (Phase 1, cli.rs).
- 🟡 Minor (medium confidence): Exit-code mapping for config-composition failures is unspecified across all four config-dependent subcommands (Phase 1 §3, Phase 4 §3).
- 🟡 Minor (medium confidence): No specified handling for a regex-valid but `u32`-overflowing `--count` value (Phase 1 §2/§3).
- 🔵 Suggestion (low confidence): `SystemClock`'s hard failure on unresolvable UTC offset is a known, pre-existing divergence from bash's silent degrade (Phase 2).

### Safety

**Summary**: This is a low-blast-radius migration: all five new subcommands are read-only against the corpus, so there is no new data-mutation or destructive-operation risk, and deleted bash files are ordinary VCS-recoverable removals. The main safety-relevant issue is that Phase 4 retires the project's only purpose-built protective mechanism for corpus-frontmatter drift (the by-name "required suite" CI gate) and replaces it with an implicit assumption that a plain `cargo test` failure is equivalently fail-closed, without confirming the new test isn't excluded from the default workspace test run the way `accelerator-visualiser` already is.

**Strengths**:
- Every new subcommand is read-only — no new destructive-write path is introduced.
- TDD-first approach captures bash-golden output before deleting the bash oracle, preserving known edge cases rather than silently "fixing" them.
- Config composition uses `LegacyPolicy::Reject`, an explicit fail-closed choice.
- Floor/registry decrements scoped phase-by-phase; the previously-hardcoded `uploads == 30` literal becomes a derived expression.

**Findings**:
- 🟡 Minor (medium confidence): Required-by-name CI gate is retired without confirming its Rust-side replacement is equally hard to silently lose (Phase 4, Removal/Tests).
- 🟡 Minor (medium confidence): Referential-integrity index silently coalesces duplicate `(type, id)` keys with no violation raised (Phase 4 §2).
- 🔵 Suggestion (medium confidence): File-list mode silently skips referential-integrity checking with no caller warning (Phase 4, Command layer / Phase 5 docs).
- 🔵 Suggestion (low confidence): No test cases enumerated for missing/corrupted `.accelerator/config.md` during config composition (Phases 1, 3, 4).

### Usability

**Summary**: The plan structures `accelerator-corpus` as a consistent noun/verb CLI mirroring the existing `vcs-cli`/`config templates` precedent, ports proven domain logic where it exists, and pays real attention to error-text and default-value parity. Its main gaps are underspecified behaviour for `frontmatter validate`'s mixed-argument case, missing discussion of `--help` discoverability, and at least one place (`linkage extract`'s bare optional positional) where the plan mirrors bash's argument shape even though "What We're NOT Doing" explicitly says CLI syntax isn't constrained to match bash.

**Strengths**:
- Noun/verb command shape consistent across all four groups, mirroring already-shipped precedents.
- Phase 2 proactively closes a pre-existing `allowed-tools` gap.
- `allowed-tools` rewritten from broad wildcards to exact subcommand grants for clarity.
- Sensible zero-config defaults preserved or improved, with specific, actionable error messages.

**Findings**:
- 🟡 Major (medium confidence): Ambiguous mode-inference for `frontmatter validate`'s multi-argument case (Phase 4 §3).
- 🔵 Minor (medium confidence): Bare optional positional argument for `linkage extract`'s source-type override mirrors bash shape unnecessarily (Phase 3 §2).
- 🔵 Minor (low confidence): Illustrative CLI snippets omit help-text doc comments (Phases 1, 3, 4).
- 🔵 Minor (low confidence): Hand-validated raw-`String` `count` argument breaks the CLI's otherwise clap-idiomatic validation pattern with no marker distinguishing it as a parity exception (Phase 1 §1).
- 🔵 Suggestion (low confidence): Combined ADR-status failure message carries forward reduced specificity from bash (Phase 1, `read_status`) — accepted by design, worth a forward-looking note.

---

## Re-Review (Pass 2) — 2026-08-07T12:40:38+00:00

**Verdict:** REVISE

The plan was substantially revised in response to Pass 1: all 8 findings behind the 8 Recommended Changes were addressed, plus every one of the 17 minor and 5 suggestion findings, walked through theme-by-theme with the plan's author. All 8 lenses were re-run in full against the revised plan. Every finding from Pass 1 is confirmed **Resolved** by its own lens with no exceptions — but the scope of the revision (a full injected-port generalisation across all four command modules, plus a ground-up redesign of `frontmatter validate`'s argument surface) introduced substantial new surface area, and that new surface area carries 6 new major findings, two of them logic bugs independently verified against the actual bash/Python source rather than inferred from the plan text alone.

### Previously Identified Issues

All 8 Major findings from Pass 1 — **Resolved**, confirmed independently by the lens that originally raised each:
- 🟡 **Correctness**: `validate_path`'s path-inferred type fallback — Resolved (now resolves strictly from frontmatter, matching bash)
- 🟡 **Correctness**: `dangling_refs` short-circuit ordering — Resolved (now gated on `NoFence`/`InvalidType`, and `DuplicateId` inherits the same gate)
- 🟡 **Compatibility**: clap exit code 2 vs bash exit 1 (ADR usage errors) — Resolved (hand-validated, exit 1 preserved)
- 🔴 **Standards** / 🔴 **Architecture**: `table_from_config` duplication + wrong crate — Resolved (delegates to `table_from_paths`, moved to `corpus-cli`)
- 🔴 **Code Quality** / 🟡 **Usability**: `frontmatter validate` silent mode switching — Resolved (explicit `--dir`/`--file`/`--checks` design, no shape-inference)
- 🟡 **Architecture** / 🟡 **Code Quality**: command-layer inline composition, no injected ports — Resolved (full `DirReader`/`FileReader`/`CorpusWalker` port generalisation, mirroring `vcs-cli`)
- 🟡 **Test Coverage**: no config-composition test coverage — Resolved (non-default-config and `LegacyPolicy::Reject` tests added to all three config-dependent phases)
- 🟡 **Test Coverage**: frontmatter-validate test breadth vs the 49-case bash suite — Resolved (explicit coverage-parity requirement stated, not just violation-code count)

All 17 Minor and 5 Suggestion findings from Pass 1 — **Resolved**. Every lens confirmed its own prior minor/suggestion findings closed with no partial resolutions or regressions; see the per-lens re-review summaries below for the specific confirmation of each.

### New Issues Introduced

#### Major

- 🔴 **Correctness** (Phase 4 §3, Index construction): The `Index`-building rule omits bash's `has_fence` gate (verified against `scripts/validate-corpus-frontmatter.sh:213-231`) — bash never indexes a fenceless file, full stop, but the plan's fallback chain (path-inferred type, filename-stem id) would still produce an index entry for one, meaning a dangling reference bash correctly flags could silently resolve under the Rust port.
- 🔴 **Correctness** (Phase 1 §5, `test_github.py` line 338): The proposed derived `uploads`-count formula is provably wrong — verified against the actual `_setup_release` fixture, it computes 22 for a case that should be 30 (and doesn't even reproduce the pre-`corpus` baseline).
- 🟡 **Test Coverage** (Phase 2): `metadata derive` has no described failure-path test, despite AC1 requiring one success and one failure path per subcommand — the one command most exposed to environmental variation (clock/timezone, VCS state) has no test evidence for its non-success exit code.
- 🟡 **Safety** (Phase 4 §6, 0007 migration call-site): The migration script's `set -e` + post-mutation validation calls (`self_validate_structural`/`self_validate_referential`) have no recovery-guidance message, unlike every sibling guard in the same file — and the newly-widened checks (`references` on by default, `DuplicateId`) make hitting this path more likely, after files are already rewritten on disk.
- 🟡 **Usability** (Phase 4 §3): `--checks references`-only can produce a silent false-clean pass (exit 0, zero output) for a structurally broken file, since a file failing `NoFence`/`InvalidType` is simply ineligible for the referential check rather than being distinguished from "checked, clean."
- 🟡 **Usability** (Phase 4 §3, `validate_path`): Unlike every sibling command-layer function, `validate_path`'s handling of `file_reader.read` returning `Ok(None)` for a nonexistent `--file` target is unspecified — risks either a panic or a misleading "no fence" report pointing the user at the wrong problem.

#### Minor / Suggestion

15 further minor findings and 6 suggestions were raised across the re-review, none blocking on their own. The recurring themes: two lenses (Architecture, Safety) independently flagged the same ambiguity in the 0007 migration script's per-file `--file` loop rewrite (single multi-flag invocation vs. N separate invocations, the latter risking silent truncation under `set -e`); two lenses (Test Coverage, Code Quality) flagged that the new `RealFs`/port-injection machinery itself has thinner direct test coverage than the logic it wires together (no dedicated `RealFs` test, an open "shared vs. duplicated" question for test stubs); and Compatibility/Standards each flagged one place where the plan introduces new, undocumented convention choices (new stricter checks not flagged as an intentional behavioural widening; per-noun/per-verb `allowed-tools` scoping with no precedent elsewhere in the plugin). Full detail in the Per-Lens Re-Review Results section below.

### Assessment

Pass 1's findings are fully and verifiably closed — this was a real, substantive revision, not a surface-level patch, and it held up under independent re-examination by all 8 lenses. The two new high-confidence Correctness findings are worth fixing before implementation regardless of appetite for a third pass: both were caught by directly reading the actual source (bash's `build_index`, the Python release-fixture test) rather than inferred from the plan's prose, so they represent genuine, verifiable bugs rather than stylistic concerns. The Safety and Usability findings around the newly-designed `frontmatter validate` surface (`--checks references`'s false-clean risk, `validate_path`'s missing-file handling, the migration script's recovery messaging) are all consequences of deliberately expanding scope beyond a literal port — real gaps, but narrow and mechanical to close. Recommend one more targeted pass addressing at minimum the 2 Correctness bugs and the `metadata derive` test gap before moving to implementation; the remaining minors can reasonably be triaged by the author's judgement given how narrow each one is.

## Per-Lens Re-Review Results

### Architecture

**Summary**: Every Pass 1 finding (inline-composed command layer, `table_from_config`'s wrong-crate placement, duplicated fallback resolution, no shared composition-root helper) is resolved specifically, not just gestured at. New: extracting `resolve_with_fallback` bundles a refactor of already-shipped `launcher` code into this plan's blast radius (minor); the mandatory whole-corpus walk on every `frontmatter validate` invocation has undiscussed scalability implications, compounded by the same per-file-loop ambiguity Safety flagged (minor); `main.rs` as sole composition root has no described modularisation strategy if a fifth noun group is ever added (suggestion, low urgency).

### Code Quality

**Summary**: Every Pass 1 finding (dual-mode CLI argument, testability gap vs. `vcs-cli`'s injected-probe pattern, duplicated config composition, `frontmatter_validation` module size) is resolved. New: "Outcome.stderr" is used loosely to describe two different error-carrying mechanisms (`Outcome` on success, `kernel::Error::Failed`'s own payload on failure) without stating this explicitly (minor); test-double sharing across command modules (`StubDirReader`/`StubFileReader`) is left as an open "or" rather than a decision (minor); `validate_file`'s ~10 orthogonal structural checks have no stated internal decomposition (suggestion); every distinct domain failure collapses into the same generic `kernel::Error::Failed` variant, which mirrors existing `vcs-cli` convention and may be an accepted tradeoff worth a one-line confirmation (minor).

### Test Coverage

**Summary**: Every Pass 1 finding (config-composition coverage, frontmatter test-breadth preservation, `table_from_config`'s own test, the `Malformed`-YAML case, the em-dash character-level assertion, unconditional-goldens clarification, quote-unwrapping/`count=0` cases) is now explicitly present. New: `metadata derive` has no described failure-path test despite AC1 requiring one (**major**); `RealFs` — the concrete adapter every command composes in production — has no dedicated test of its own, only indirect exercise via golden tests (minor); `DuplicateId`'s short-circuit gate isn't given its own explicit test the way `dangling_refs`'s is (minor); `run_next_number`'s ADR-basename-filtering regex isn't named among the stub-based unit tests (minor); `table_from_config`'s unmatched-doc-type-name test case doesn't state its expected assertion (suggestion).

### Correctness

**Summary**: The Pass 1 bash-quirk-preservation discipline (quote-stripping order, empty-status success, the `--count` overflow guard) holds and is each pinned with a dedicated test. New, both verified directly against source: `Index` construction omits bash's `has_fence` gate, a real referential-integrity divergence (**major**, high confidence); the illustrative `uploads`-count formula in the Phase 1 registration-checklist edit is provably wrong even against the current baseline (**major**, high confidence); no overflow guard on parsing *existing* ADR directory basenames' digit run, asymmetric with the care taken for `--count` (minor).

### Standards

**Summary**: Every Pass 1 finding (`commands/` layout, `allowed-tools` wildcarding inconsistency, `table_from_config` duplication) is resolved, and nearly every structural choice is grounded in cited, verified precedent. New: the fine-grained per-noun/per-verb `allowed-tools` scoping introduced to resolve Pass 1's finding has no precedent anywhere else in the plugin, which wildcards the whole sub-binary token (`accelerator config *`, `accelerator vcs *`) uniformly — worth an explicit note that this is a conscious successor pattern, or a fallback to the coarser convention (minor); the new `cli/corpus-cli/src/config.rs` module shares its name with the `config` crate it wraps, following one of two inconsistent existing precedents in the codebase (suggestion).

### Compatibility

**Summary**: All 3 Pass 1 findings (clap exit-code conflict, unspecified `ConfigError` mapping, `--count` overflow) are resolved, the latter two via the new shared `config::compose` helper. New: the new stricter Phase 4 failure modes (`DuplicateId`, `Malformed`-folded-into-`NoFence`) aren't flagged as an intentional behavioural widening the way the checks-default change already is (minor); `linkage extract`'s required `file` argument relies on an undocumented clap/bash exit-code coincidence — verified against `scripts/linkage-parser.sh:392-396`, bash already exits 2 here, so no hand-validation is actually needed, but the plan doesn't say so (suggestion).

### Safety

**Summary**: Every Pass 1 finding (CI-gate replacement, duplicate-id blind spot, file-list-mode referential-integrity gap, missing config-failure tests) is resolved, several with genuine improvements over bash (fail-closed `LegacyPolicy::Reject`, panic-proofed `--count` overflow, `DuplicateId` closing a real bash blind spot). New: the 0007 migration script's `set -e` + post-mutation validation calls have no recovery-guidance message, unlike every sibling guard in the same file, and the newly-widened checks make hitting this more likely (**major**); the per-file `--file` loop rewrite is ambiguous between one multi-flag invocation and N separate invocations, the latter risking silent validation truncation under `set -e` (minor); the CI-gate replacement relies on one-time manual confirmation rather than a durable automated invariant (minor).

### Usability

**Summary**: Every Pass 1 finding (`linkage extract`'s positional, missing `--help` doc-comments, unmarked `count` parity exception, ambiguous mode-inference) is resolved, the last via the new explicit `--checks`/`--dir`/`--file` design. New: `--checks references`-only can silently produce a false-clean result with no distinguishing output for an ineligible file (**major**); `validate_path`'s handling of a missing `--file` target is unspecified, unlike every sibling command-layer function (**major**); the new `--checks` flag's clap-native exit code (2) sits inconsistently alongside the hand-validated bash-parity exit code (1) used elsewhere in the same binary (minor); `DuplicateId`'s violation message doesn't specify identifying the colliding file, even though the data is already available at the emission site (minor).

## Final Disposition — 2026-08-07T13:33:11+00:00

All 6 new Major findings from Pass 2's re-review are addressed:

- **`Index` construction now indexes only `Parsed` frontmatter** — `Malformed` and `Absent` are both excluded, a deliberate correctness rule (not a bash-parity port) established after discussion: a file's type/id should only be trusted as a reference target when the file itself successfully declares them, keeping the `Index` consistent with `validate_path`'s own treatment of `Malformed` as `NoFence`.
- **The `uploads`-count formula is corrected** to `len(_PLATFORMS) * 3 + 2 + len(DISPATCHED_SUBBINARIES) * len(_PLATFORMS) * 2`, verified against the actual `_setup_release` fixture and sanity-checked against both the pre- and post-`corpus` baseline values.
- **`metadata derive` now has a failure-path test** — `run_derive` takes an already-resolved `Result<ArtifactMetadata, ClockError>` (computed in `main.rs`) so its failure branch is unit-testable via a new `ClockError::new` constructor, without forcing a real host tzdata failure. The plan also now documents that bash's own script has no defined failure exit path at all.
- **The 0007 migration script's validation calls** now get the same `log_warn`/VCS-revert guard every sibling failure branch in that script already has, and the per-file `--file` loop is specified as a single multi-flag invocation rather than N separate spawns `set -e` could silently truncate.
- **`--checks references`-only now emits a distinct `SKIPPED` line** (not a violation, doesn't affect the exit code) for a file ineligible for the check that ran, closing the false-clean-result risk.
- **`validate_path`'s missing-file handling is now fully specified** — verified against bash source that a nonexistent file reports `NO-FENCE` (bash treats "doesn't exist" identically to "no fence"), matched exactly rather than left to chance.

The remaining minor findings and suggestions from Pass 2 (an `Outcome`/`kernel::Error` message-routing clarification, test-stub sharing left as an open question, `RealFs`'s own test coverage, a couple of undocumented convention choices, and similar narrow items) are accepted as-is at the plan author's judgement — each is independently addressable during implementation without materially affecting the plan's soundness.

**Final Verdict: APPROVE.** The plan is ready for implementation.

---
*Review generated by /accelerator:review-plan*
