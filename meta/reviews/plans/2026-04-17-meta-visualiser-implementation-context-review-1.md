---
date: "2026-04-29T17:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, security, correctness, compatibility, portability, safety, usability]
review_pass: 3
status: complete
---

## Plan Review: Meta Visualiser v1 — Implementation Context and Phasing

**Verdict:** REVISE

The plan demonstrates exceptional architectural thinking — clean module boundaries, a well-chosen trait-based file-driver abstraction, comprehensive handling of real-world data messiness (absent/malformed frontmatter, slug collisions), and careful adherence to existing plugin conventions. Security posture is strong with localhost-only binding, SHA-256 binary verification, path-escape guards, and atomic writes. However, several correctness-critical gaps need resolution: a TOCTOU window in the write path undermines the ETag conflict-detection guarantee, an undocumented 12th path key (`review_tickets`) means the indexer will silently miss documents, the broadcast channel overflow can silently drop events without client recovery, and the release ordering has no atomicity guarantee between manifest commit and binary upload.

### Cross-Cutting Themes

- **TOCTOU in the write path** (flagged by: correctness, safety, security) — The PATCH endpoint checks the cached ETag rather than computing a fresh hash at read time. Between the cache check and the file read, external edits can land undetected, undermining the very conflict-detection guarantee the ETag system exists to provide.
- **Broadcast overflow with no client recovery** (flagged by: architecture, correctness) — The `tokio::sync::broadcast` channel silently drops events for slow consumers but the plan doesn't describe how the client detects missed events or recovers without a full reconnect.
- **Missing `review_tickets` path key** (flagged by: correctness, compatibility) — The current codebase has evolved to include a 12th path key that the plan doesn't account for, meaning documents in `meta/reviews/tickets/` would be invisible to the visualiser.
- **First-run experience gaps** (flagged by: usability, portability) — The binary download on first invocation has insufficient progress feedback, `shasum` availability varies across Linux, and error recovery guidance is minimal.

### Tradeoff Analysis

- **Error handling timing vs. shipping velocity**: The usability lens identifies that deferring all error UX to Phase 10 creates a poor dogfooding experience during Phases 5-9, while the phasing rationale explicitly optimizes for landing earlier phases quickly. Recommendation: pull forward the three most critical error items (init detection, SSE reconnect, keyboard focus) to Phase 5 while leaving polish in Phase 10.
- **Security depth vs. threat model appropriateness**: The security lens flags TOCTOU races in canonicalize, missing CORS specifics, and lack of binary signing. Most of these are appropriate for a localhost-only, single-user tool. The TOCTOU in the write path (ETag check) is the one that crosses the line from "theoretical" to "functionally incorrect."

### Findings

#### Critical

None.

#### Major

- 🟡 **Correctness/Safety/Security**: TOCTOU between cached ETag check and file write in PATCH endpoint
  **Location**: Phase 8: Kanban write path
  The write path checks the *cached* ETag against If-Match, then reads the file. Between these steps, an external edit within the 100ms debounce window goes undetected, silently overwriting concurrent changes.

- 🟡 **Correctness/Compatibility**: Missing `review_tickets` path key creates silent data loss
  **Location**: Section 2: Path & config resolution; Phase 3: Indexer
  The current `config-read-path.sh` includes a 12th key (`review_tickets` → `meta/reviews/tickets`) not mentioned in the plan. Documents there would be invisible to the visualiser.

- 🟡 **Correctness**: Debounce HashMap loses events on rapid successive writes
  **Location**: Phase 4: SSE hub and notify watcher
  Aborting a debounce task mid-index-update (after it passes the sleep) leaves the indexer in an inconsistent state or swallows the broadcast event.

- 🟡 **Architecture**: Indexer conflates data store, event source, and computation responsibilities
  **Location**: Phase 3: FileDriver, Indexer, and read-only API
  The Indexer is assigned too many reasons to change (data structure, clustering algorithm, event propagation), making it the system's gravity well as it evolves.

- 🟡 **Safety**: Release manifest commit and binary upload are not atomic
  **Location**: Follow-up research, Gap 3: Release flow ordering
  A failure between tag push and binary upload leaves users with a manifest pointing to non-existent assets.

- 🟡 **Safety**: Stale server-info.json after unclean shutdown with PID recycling
  **Location**: Phase 2: Server bootstrap and lifecycle
  If the process is SIGKILL'd and the OS recycles the PID, the preprocessor could incorrectly declare the server alive.

- 🟡 **Security**: CORS configuration unspecified beyond 'same-origin'
  **Location**: Design spec: Non-functional — Security section
  Without explicit CORS headers, any webpage could potentially make API requests to the visualiser if it discovers the dynamic port.

- 🟡 **Security**: Binary download relies solely on committed checksums — no independent trust root
  **Location**: Phase 2: binary download flow
  A repository compromise allows updating both checksums.json and binaries in a coordinated attack.

- 🟡 **Portability**: `shasum` unavailable on many Linux distributions
  **Location**: D8: Binary distribution, launch-server.sh
  The plan uses `shasum -a 256` but Linux often only has `sha256sum`. macOS has `shasum` but not `sha256sum`.

- 🟡 **Usability**: Error handling and accessibility deferred too late
  **Location**: Phase 10
  Phases 5-9 ship user-facing UI with no graceful degradation, no init-not-run detection, and no keyboard accessibility.

- 🟡 **Usability**: First-run download lacks progress indication and retry guidance
  **Location**: Phase 2, D8
  A silent multi-second hang or opaque failure on the very first user interaction sets a negative tone.

#### Minor

- 🔵 **Architecture**: Templates virtual DocType creates a bifurcated code path in the FileDriver
  **Location**: Phase 3
  Every component handling 'all doc types' must branch on whether the type is templates. Consider a DocTypeProvider trait.

- 🔵 **Architecture**: Broadcast channel back-pressure may cause silent data loss for slow consumers
  **Location**: Phase 4
  No sequence number or generation counter to let the client detect missed events without full reconnect.

- 🔵 **Architecture**: Patcher module's field allowlist hardcoded rather than separated from YAML mechanics
  **Location**: Phase 8
  Separating FieldPolicy from YamlPatcher would make v2 extensions (more writable fields) cleaner.

- 🔵 **Correctness**: Review slug regex ambiguous when slug itself contains `-review-` followed by digits
  **Location**: Phase 3: Slug derivation
  Specify that the suffix strip matches the *last* occurrence of `-review-\d+` anchored to end of stem.

- 🔵 **Correctness**: Owner-PID as grandparent-of-grandparent may not correctly identify the harness
  **Location**: Phase 2
  Process tree depth may vary. Pass the harness PID explicitly via env var (as superpowers does with BRAINSTORM_OWNER_PID).

- 🔵 **Correctness**: Slug collision across unrelated doc types produces false clusters
  **Location**: Phase 6: Lifecycle clusters
  Common short slugs like `configuration` will collide. Acknowledged as authoring discipline; consider UI indicator.

- 🔵 **Security**: No rate limiting on PATCH endpoint enables rapid disk churn
  **Location**: Phase 8
  A per-path rate limiter (one write/second/path) would protect against buggy frontends and local scripts.

- 🔵 **Security**: No Content-Type validation documented for PATCH body
  **Location**: Phase 8
  Ensure axum's `Json<T>` extractor is used (returns 415 without application/json) for defence-in-depth alongside CORS.

- 🔵 **Safety**: Unbounded debounce HashMap during mass file operations
  **Location**: Phase 4
  If pending entries exceed a threshold, switch to full-rescan rather than tracking individual paths.

- 🔵 **Safety**: Idle timeout definition ambiguous with SSE keep-alive connections
  **Location**: Phase 2
  Define idle as 'no new HTTP requests AND no active SSE subscribers' so the server stays alive while someone is viewing.

- 🔵 **Portability**: `jq` dependency undocumented for plugin version extraction
  **Location**: D8, Phase 2
  Document as prerequisite or extract version with POSIX-compatible grep/sed.

- 🔵 **Usability**: CLI wrapper installation requires manual symlink with no guided setup
  **Location**: D1, Phase 1
  Include a one-liner in slash command output showing how to install the CLI.

- 🔵 **Usability**: Checksum mismatch error gives no recovery steps
  **Location**: Phase 2
  Include expected/actual checksums, release URL, and mention of ACCELERATOR_VISUALISER_BIN.

- 🔵 **Usability**: Owner-PID shutdown produces silent browser disconnection
  **Location**: Phase 2
  Add a 'server-shutdown' SSE event before exit so the frontend can show a clear message.

#### Suggestions

- 🔵 **Architecture**: Testing phase positioned after all implementation reduces architectural feedback — add a lightweight testability checkpoint at end of Phase 4.
- 🔵 **Architecture**: Deep nesting of visualiser sources creates unusual Cargo topology with fragile relative paths to frontend/dist/.
- 🔵 **Correctness**: Broadcast overflow should inject 'resync-needed' event rather than silently dropping.
- 🔵 **Portability**: Release pipeline coupled to single macOS host — document CI-based alternative for Phase 12.
- 🔵 **Usability**: Deep-link URLs use :filename but it's not a filename — use :fileSlug consistently.
- 🔵 **Usability**: In-progress kanban column needs empty-state guidance since no skill has ever written that status.

### Strengths

- ✅ FileDriver trait provides clean abstraction boundary for future alternative implementations
- ✅ Config resolution delegated entirely to bash preprocessor — excellent functional core / imperative shell separation
- ✅ Three-state frontmatter model correctly handles messy real-world data without treating absence as an error
- ✅ Two-type review split elegantly absorbs physical directory nesting into the type system
- ✅ Zero runtime dependency for end users with transparent binary acquisition and SHA-256 verification
- ✅ Instance reuse via PID detection provides instant re-invocation with least-surprise behaviour
- ✅ Three-tier template visualisation makes configuration resolution visible rather than opaque
- ✅ Atomic file writes via tempfile+rename prevents partial-write corruption
- ✅ ETag-based conflict detection follows HTTP semantics correctly
- ✅ Owner-PID watch provides graceful self-cleaning on harness death
- ✅ POSIX-only CLI wrapper pattern already validated and consistent with existing scripts
- ✅ musl for truly static Linux binaries eliminates glibc version coupling entirely

### Recommended Changes

1. **Close the TOCTOU in the write path** (addresses: TOCTOU ETag finding)
   In Phase 8's PATCH handler, compute SHA-256 of the file at read time and compare directly against the `If-Match` header, rather than checking the cached ETag.

2. **Verify `review_tickets` path key status** (addresses: missing path key finding)
   Confirm whether `review_tickets` was added to `config-read-path.sh` after the research snapshot. If legitimate, add `ticket-reviews` as an 11th DocType; if not, document the exclusion.

3. **Handle broadcast overflow on the server side** (addresses: debounce lost events, broadcast overflow)
   When `tokio::sync::broadcast` returns `RecvError::Lagged`, inject a synthetic `invalidate-all` event. For debouncing, use a timestamp-check pattern instead of aborting tasks.

4. **Add a health-check to PID reuse logic** (addresses: stale server-info.json)
   Before declaring a server alive based on PID, attempt a `GET /api/types` with a short timeout.

5. **Use portable SHA-256 in launch-server.sh** (addresses: shasum portability)
   Use `sha256sum` on Linux (check first), fall back to `shasum -a 256` on macOS.

6. **Specify CORS explicitly** (addresses: CORS configuration finding)
   Configure the axum router to reject cross-origin requests. Use `tower-http`'s CORS layer.

7. **Pull forward critical error UX to Phase 5** (addresses: deferred error handling)
   Move init-not-run detection, SSE reconnect with backoff, and basic keyboard focus into Phase 5.

8. **Add progress feedback to first-run download** (addresses: first-run UX)
   Use `curl --progress-bar` for download progress. On failure, suggest retry and mention ACCELERATOR_VISUALISER_BIN.

9. **Define idle timeout as "no requests AND no SSE subscribers"** (addresses: idle timeout ambiguity)
   The server stays alive while someone is viewing.

10. **Document release script failure handling** (addresses: release atomicity)
    Upload binaries under a draft release, verify checksums match manifest, then promote and push tag.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan demonstrates strong architectural thinking with clear module boundaries, a well-chosen trait-based file-driver abstraction, and careful adherence to existing plugin conventions. The main concerns are around the tight coupling between the release/distribution pipeline and the runtime system, the single-process indexer acting as both data store and event source without clear separation, and a few places where evolutionary fitness could be improved by introducing intermediate abstractions.

**Strengths**:
- The FileDriver trait provides a clean abstraction boundary that enables future alternative implementations without touching callers
- Config resolution is correctly delegated entirely to the bash preprocessor, keeping the Rust server as a pure functional core
- The three-state frontmatter model (parsed, absent, malformed) is well-designed
- The two-type review split elegantly absorbs the physical directory nesting into the type system
- The plan leverages existing meta/tmp, config-read-path.sh, and SKILL.md conventions — excellent consistency
- Owner-PID watch pattern provides graceful self-cleaning on harness death

**Findings**:

1. **Major** (high confidence): Indexer conflates data store, event source, and computation responsibilities
   - Location: Phase 3: FileDriver, Indexer, and read-only API
   - The Indexer has at least three reasons to change: data structure changes, clustering algorithm changes, and event propagation changes. Consider splitting into IndexStore, ClusterComputer, and EventProcessor.

2. **Major** (medium confidence): Binary acquisition in the critical startup path creates a fragile dependency chain
   - Location: Phase 2: Server bootstrap and lifecycle; D8
   - First-run reliability is entirely dependent on GitHub Releases availability. Consider a separate preparatory command and user-global binary cache.

3. **Minor** (high confidence): Templates virtual DocType creates a bifurcated code path
   - Location: Phase 3
   - Consider a DocTypeProvider trait with DirectoryDocTypeProvider and TemplateDocTypeProvider implementations.

4. **Minor** (medium confidence): Broadcast channel back-pressure may cause silent data loss
   - Location: Phase 4
   - Include a sequence number in SSE events; on Lagged, send a 'resync-needed' event.

5. **Minor** (high confidence): Patcher module's field allowlist hardcoded rather than configurable
   - Location: Phase 8
   - Separate FieldPolicy from YamlPatcher for v2 extensibility.

6. **Suggestion** (medium confidence): Deep nesting creates unusual Cargo project topology
   - Location: D2
   - Relative path coupling is the cost of colocation; document in build.rs.

7. **Suggestion** (medium confidence): Testing phase positioned after all implementation phases
   - Location: Phase 11
   - Add a lightweight testability checkpoint at end of Phase 4.

### Security

**Summary**: The plan demonstrates solid security thinking with localhost-only binding, SHA-256 checksum verification, path traversal guards, field/value allowlists, and atomic writes. Gaps exist around TOCTOU races, CORS hardening, supply chain concerns, and rate limiting.

**Strengths**:
- Binary distribution uses committed SHA-256 checksums verified before execution
- Path traversal prevention via canonicalize + prefix check handles symlink escapes
- PATCH endpoint uses both field and value allowlists — defence in depth
- Atomic file writes prevent partial-write corruption
- Localhost-only binding significantly reduces attack surface
- No shell execution from within the Rust server eliminates command injection

**Findings**:

1. **Major** (high confidence): HTTPS download without signature verification beyond SHA-256
   - Location: Phase 2: binary download flow
   - Repository compromise allows coordinated update of checksums and binaries. Accept as risk given threat model, or add minisign.

2. **Major** (medium confidence): TOCTOU race between canonicalize and file operations
   - Location: Phase 3: FileDriver
   - Residual risk acceptable given localhost + same-user threat model; document it.

3. **Major** (high confidence): No rate limiting on PATCH endpoint
   - Location: Phase 8
   - Add per-path rate limiter (one write/second/path).

4. **Major** (medium confidence): CORS configuration unspecified
   - Location: Design spec: Non-functional
   - Explicitly configure CORS to reject cross-origin requests; add custom header check.

5. **Minor** (high confidence): Download permissions set before verification in edge cases
   - Location: Phase 2: launch-server.sh
   - Ensure chmod +x happens after atomic rename, not before.

6. **Minor** (medium confidence): No Content-Type validation on PATCH body
   - Location: Phase 8
   - Ensure axum's Json<T> extractor is used (returns 415 without application/json).

7. **Minor** (medium confidence): SSE leaks file paths to any localhost client
   - Location: Phase 4
   - Accepted trade-off of no-auth localhost; document as known surface.

8. **Minor** (high confidence): Log rotation without retention limit
   - Location: Phase 10
   - Specify max retained rotated files (e.g., 2).

### Correctness

**Summary**: The plan is largely sound on correctness fundamentals. The frontmatter state machine, ETag semantics, and conflict handling are well-specified. However, the debounce map has a potential lost-update race, the path-key enumeration is stale, and the PATCH endpoint's ETag check has a TOCTOU window.

**Strengths**:
- Three-state frontmatter model correctly handles real-world data
- ETag-based conflict detection follows HTTP semantics correctly
- Atomic write via tempfile+rename is the correct primitive
- Owner-PID watch using kill(pid, 0) is sound
- Review suffix strip regex is essential and correctly identified

**Findings**:

1. **Major** (high confidence): Debounce HashMap loses events on rapid successive writes
   - Location: Phase 4: SSE hub and notify watcher
   - Use timestamp-check pattern instead of aborting tasks.

2. **Major** (medium confidence): TOCTOU between cached ETag check and file read in write_frontmatter
   - Location: Phase 8: Kanban write path
   - Compute SHA-256 at read time rather than relying on cached ETag.

3. **Major** (high confidence): Plan enumerates 11 path keys but config-read-path.sh defines 12
   - Location: Section 2; Phase 3
   - Verify `review_tickets` status; add as DocType or document exclusion.

4. **Minor** (medium confidence): Review slug regex ambiguous with `-review-` in slug
   - Location: Phase 3: Slug derivation
   - Anchor suffix to end: `^\d{4}-\d{2}-\d{2}-(.+)-review-\d+$`.

5. **Minor** (medium confidence): Slug collision produces false clusters
   - Location: Phase 6
   - Acknowledged as authoring discipline; consider UI indicator.

6. **Minor** (high confidence): Broadcast channel overflow silently drops events
   - Location: Phase 4
   - Inject 'invalidate-all' event on Lagged rather than silent drop.

7. **Minor** (medium confidence): Owner-PID as grandparent may misidentify harness
   - Location: Phase 2
   - Pass harness PID explicitly via env var.

### Compatibility

**Summary**: The plan demonstrates strong compatibility alignment between Rust server and TypeScript frontend. The key concern is the newly-added `review_tickets` path key not accounted for. Minor discrepancies exist between spec and implementation (version header name, virtual field optionality, template endpoint paths).

**Strengths**:
- Wire-format types precisely mirrored between Rust and TypeScript
- ETag format consistently used across all components
- config.json uses deny_unknown_fields catching drift at startup
- SSE event types are tagged unions with clear TypeScript counterparts
- checksums.json format clearly separated from ETag format

**Findings**:

1. **Major** (high confidence): Missing review_tickets path key creates silent contract gap
   - Location: Section 2
   - Documents in meta/reviews/tickets/ invisible to the visualiser.

2. **Minor** (high confidence): Version header name diverges from spec
   - Location: Phase 10
   - Align spec and implementation on the header name.

3. **Minor** (high confidence): DocType.virtual always-present vs spec's optional contract
   - Location: Phase 3
   - Update spec to remove the `?` marker.

4. **Minor** (medium confidence): IndexEntry has fields not in spec
   - Location: Phase 3
   - Update spec to include frontmatterState, ticket, bodyPreview.

5. **Minor** (medium confidence): Templates endpoint at /api/templates not /api/docs?type=templates
   - Location: Phase 3
   - Update spec to document actual endpoint paths.

### Portability

**Summary**: The plan demonstrates strong portability awareness. Cross-compilation strategy, POSIX-only scripts, and per-arch binary distribution are well-considered. A few implementation details could create friction on specific platforms.

**Strengths**:
- Four-architecture target matrix with musl for truly static Linux binaries
- POSIX-only CLI wrapper consistent with existing scripts
- Binary distribution means zero build toolchain for end users
- ACCELERATOR_VISUALISER_BIN provides escape hatch for air-gapped environments
- notify crate correctly identified as cross-platform file-watching abstraction

**Findings**:

1. **Major** (high confidence): shasum availability varies across Linux distributions
   - Location: D8, Phase 2
   - Use sha256sum on Linux, shasum -a 256 on macOS, with fallback.

2. **Minor** (medium confidence): jq dependency undocumented
   - Location: D8, Phase 2
   - Document as prerequisite or use POSIX-compatible extraction.

3. **Minor** (medium confidence): Release build coupled to single macOS host
   - Location: Phase 12
   - Document CI alternative for Phase 12 planning.

4. **Minor** (high confidence): uname -m normalisation needs explicit dual-form handling
   - Location: D8
   - Ensure case statement covers both aarch64 and arm64.

5. **Suggestion** (medium confidence): nix crate ties to Unix-only semantics
   - Location: Phase 2
   - No action needed; Windows explicitly out of scope.

6. **Suggestion** (low confidence): curl availability assumption
   - Location: Phase 2
   - Acceptable; env-var override covers edge case.

### Safety

**Summary**: Strong safety awareness in core write path and process lifecycle. Gaps in release ordering atomicity, TOCTOU in write path, and incomplete handling of partial state transitions during shutdown and binary acquisition.

**Strengths**:
- Atomic file writes via tempfile+rename prevents corruption
- ETag conflict detection prevents lost updates
- Strict field/value allowlists limit accidental damage
- Path-escape guard prevents writes outside configured directories
- Owner-PID watch ensures self-termination
- Binary checksum mismatch leads to delete + abort (correct fail-safe)
- Graceful shutdown sequence ensures clean state transitions
- Three-state frontmatter means bad data never crashes the system

**Findings**:

1. **Major** (high confidence): TOCTOU window between ETag check and atomic write
   - Location: Phase 8
   - Compute SHA-256 at read time, not from cache.

2. **Major** (medium confidence): Release manifest and binary upload not atomic
   - Location: Gap 3
   - Upload under draft release, verify, then promote.

3. **Major** (medium confidence): Stale server-info.json after unclean shutdown with PID recycling
   - Location: Phase 2
   - Add health-check HTTP request before declaring server alive.

4. **Minor** (high confidence): Unbounded debounce HashMap during mass file operations
   - Location: Phase 4
   - Switch to full-rescan when pending entries exceed threshold.

5. **Minor** (high confidence): Idle timeout definition ambiguous with SSE connections
   - Location: Phase 2
   - Define as "no requests AND no SSE subscribers."

6. **Minor** (medium confidence): Partial download file state on interruption
   - Location: Phase 2
   - Always download to .part, verify, then atomic rename.

7. **Suggestion** (medium confidence): Log rotation without retention count
   - Location: Phase 10
   - Specify max 3 rotated files.

### Usability

**Summary**: Thoughtful developer experience with strong defaults. Main concerns are first-run latency/feedback, CLI installation ceremony, error recoverability, and deferred error handling creating poor early-adopter experience.

**Strengths**:
- Zero runtime dependency for end users
- Instance reuse is instant and least-surprise
- Dev override (ACCELERATOR_VISUALISER_BIN) is a clean escape hatch
- Progressive disclosure well-modelled (slash command → CLI → env vars)
- Consistent use of existing patterns makes the visualiser navigable
- Three-tier template visualisation is a genuine usability win
- Strong ETag conflict handling with clear UI feedback

**Findings**:

1. **Major** (high confidence): Error handling and accessibility deferred too late
   - Location: Phase 10
   - Pull init detection, SSE reconnect, keyboard focus into Phase 5.

2. **Major** (medium confidence): First-run download lacks progress and retry guidance
   - Location: Phase 2, D8
   - Use curl --progress-bar; suggest retry on failure.

3. **Minor** (high confidence): CLI wrapper requires manual symlink
   - Location: D1, Phase 1
   - Include install one-liner in slash command output.

4. **Minor** (medium confidence): Checksum mismatch error gives no recovery steps
   - Location: Phase 2
   - Include expected/actual checksums and ACCELERATOR_VISUALISER_BIN mention.

5. **Minor** (medium confidence): Owner-PID shutdown produces silent browser disconnection
   - Location: Phase 2
   - Add 'server-shutdown' SSE event before exit.

6. **Suggestion** (medium confidence): Deep-link URLs use :filename but it's not a filename
   - Location: Phase 5
   - Use :fileSlug consistently.

7. **Suggestion** (low confidence): In-progress kanban column needs empty-state guidance
   - Location: Phase 7
   - Add empty-state message explaining no tickets have used this status yet.

---

## Re-Review (Pass 2)

**Verdict:** COMMENT

**Summary**: All 11 major findings from Pass 1 have been addressed. The plan now includes TOCTOU mitigation (fresh SHA-256 at read time), review_tickets/ticket-reviews throughout, timestamp-based debounce with threshold-triggered full-rescan, broadcast overflow recovery via invalidate-all injection, PID health-check before reuse, explicit CORS rejection, portable SHA-256 with platform detection, critical error UX pulled forward to Phase 5, idle timeout defined as "no requests AND no SSE subscribers", draft-release atomicity strategy, and download progress feedback. No critical or major findings remain — only minor residuals and new suggestions.

### Resolution Status

| Pass 1 Finding | Status | Notes |
|---|---|---|
| TOCTOU in write path | **Resolved** | Phase 8 now computes fresh SHA-256 at read time |
| Missing review_tickets | **Resolved** | Added throughout: path key counts, DocTypeKey union, Phase 3, Phase 4 watchers |
| Debounce lost events | **Resolved** | Switched to timestamp-check pattern; threshold triggers full-rescan |
| Broadcast overflow | **Resolved** | invalidate-all injection on RecvError::Lagged specified |
| Release atomicity | **Resolved** | 7-step draft-release strategy in Phase 12 |
| PID recycling | **Resolved** | Health-check HTTP request before reuse |
| CORS unspecified | **Resolved** | tower-http CorsLayer with explicit rejection |
| Binary signing | **Accepted** | Acknowledged as acceptable risk for localhost/single-user threat model |
| shasum portability | **Resolved** | Platform-aware fallback (sha256sum on Linux, shasum on macOS) |
| Error handling deferred | **Resolved** | Critical UX items pulled to Phase 5 |
| First-run download UX | **Resolved** | curl --progress-bar + retry guidance |
| Indexer decomposition | **Accepted** | Acknowledged as evolutionary concern; current scope appropriate for v1 |

### Residual Minor Findings

- 🔵 **Correctness**: Line 61 still says "11" path keys; should be "12" to match current config-read-path.sh (review_tickets added 2026-04-24)
- 🔵 **Correctness**: Phase 4 says "11 source dirs" (line 1040); should be "10" — templates is virtual, not a watched directory
- 🔵 **Correctness**: Consistency-check section (lines 1486-1488) still references "11 path keys" — stale since review_tickets addition
- 🔵 **Safety**: Debounce HashMap entry cleanup unspecified — entries for paths that complete successfully should be removed to prevent unbounded growth during long server sessions
- 🔵 **Safety**: Orphaned `.part` files from interrupted downloads not explicitly cleaned up on next launch
- 🔵 **Compatibility**: Design spec (`meta/specs/2026-04-17-meta-visualisation-design.md`) not yet updated to include `ticket-reviews` DocTypeKey
- 🔵 **Compatibility**: Spec uses `:filename` in deep-link URLs; plan uses `:fileSlug` — inconsistency remains

### New Suggestions

- 💡 Phase 12 release: tag-push-before-promotion edge case — if the tag push succeeds but promotion fails, the tag points at binaries that aren't yet public. Consider promoting first, then pushing the tag.
- 💡 `jq` dependency for plugin version extraction is undocumented as a prerequisite in Phase 2.
- 💡 In-progress kanban column should include empty-state guidance text since no skill currently writes that status value.
- 💡 Debounce HashMap entries should specify a TTL or cleanup-on-success policy.

### Assessment

The plan is now in strong shape for implementation. All correctness-critical and safety-critical gaps from Pass 1 have been addressed with well-specified solutions. The residual findings are cosmetic count mismatches (easily fixed) and minor operational gaps that won't block implementation. The plan is acceptable as-is — see minor findings above for optional polish.

---
*Re-review generated by /review-plan (Pass 2)*

---

## Re-Review (Pass 3)

**Verdict:** COMMENT

**Summary**: All correctness-critical and safety-critical issues remain resolved. The count fixes from Pass 2 (path keys 12, source dirs 10) are correct in operative plan sections. Pass 3 surfaces two distinct categories of residual findings: (1) editorial inconsistencies within the plan document itself (stale headings, counts, and a "Node" reference that should say "Rust"), and (2) a spec-plan divergence where the design spec needs updating to include `ticket-reviews` but the plan itself is correct.

### Resolution Status from Pass 2

All Pass 2 residual findings verified:
- **Line 61 path key count**: Fixed (now says "12")
- **Line 1040 source dirs**: Fixed (now says "10")
- **Lines 1486-1488 consistency section**: Fixed (now says "12")
- **Line 1481 ADR impact note**: Fixed (now says "12 path keys or the 11 DocTypes")

### New Findings

#### Critical (spec-only, not plan)

- 🔴 **Compatibility**: Design spec DocTypeKey union omits `ticket-reviews`
  **Location**: Spec (not the plan) — DocTypeKey, LifecycleCluster.completeness, preprocessor path list, timeline ordering
  The spec was last updated 2026-04-18; `review_tickets` was added to config-read-path.sh on 2026-04-24. The plan correctly includes `ticket-reviews` throughout, but the spec is stale. **Action**: update the spec before implementation begins.

#### Major (spec-only)

- 🟡 **Compatibility**: LifecycleCluster.completeness interface missing `hasTicketReview`
  **Location**: Spec LifecycleCluster interface
  The plan's Phase 6 lifecycle ordering includes ticket-reviews but neither document adds `hasTicketReview` to the completeness struct.

- 🟡 **Compatibility**: Lifecycle timeline ordering not specified in spec for ticket-reviews
  **Location**: Spec timeline definition
  Plan places ticket-reviews after plan-review, before validation. Spec has no position for it.

- 🟡 **Compatibility**: Spec preprocessor path list omits review_tickets
  **Location**: Spec preprocessor step 2
  Lists 10 path keys; should be 11 (or 12 if tmp is included).

#### Minor (plan editorial)

- 🔵 **Correctness**: Summary says "Seven design decisions" — should be "Ten" (D1-D10 exist)
- 🔵 **Correctness**: Line 66 says "two separate DocTypes" — should be "three" (ticket-reviews added)
- 🔵 **Correctness**: D5 heading says "two separate DocTypes" — should be "three"
- 🔵 **Correctness**: Phase 3 claims wire-format "matches the TS union in the spec" — spec lacks ticket-reviews
- 🔵 **Correctness**: Phase 3 slug suffix-strip lists only plan-reviews and pr-reviews, omits ticket-reviews
- 🔵 **Architecture**: Phasing rationale says "before any Node is written" — should say "Rust"
- 🔵 **Correctness**: Line 1350 says "11 path keys" — should note current count is 12
- 🔵 **Correctness**: Follow-up inconsistency #4 is stale (Phase 6 ordering already fixed)
- 🔵 **Correctness**: Follow-up inconsistency #1 says "eight" decisions — now ten
- 🔵 **Portability**: D8 step 4 references only `shasum -a 256` without mentioning the portable wrapper from Phase 2
- 🔵 **Usability**: Checksum-mismatch error doesn't mention ACCELERATOR_VISUALISER_BIN as recovery path
- 🔵 **Safety**: Debounce HashMap entry cleanup (successful paths) unspecified
- 🔵 **Safety**: Orphaned .part files not explicitly cleaned on next launch

#### Suggestions

- 💡 Connection-status indicator in UI during SSE disconnect (usability)
- 💡 Conflict toast should include actionable guidance text (usability)
- 💡 Phase 2 placeholder HTML page for pre-frontend phases (usability)
- 💡 Security-relevant events (path-escape, PATCH rejections) should emit WARN-level traces (security)
- 💡 Markdown renderer should maintain sanitization defaults (no raw HTML) for defense-in-depth (security)
- 💡 Consider `GET /health` endpoint for liveness checks independent of API versioning (compatibility)

### Assessment

**Verdict remains COMMENT.** The plan's operative implementation sections (Phases 1-12) are internally consistent and implementation-ready. The findings fall into two categories:

1. **Spec update needed** (critical/major): The design spec must be updated to include `ticket-reviews` before implementation. This is a spec maintenance task, not a plan deficiency — the plan correctly anticipated the addition.

2. **Plan editorial polish** (minor): Stale counts in headings, the summary paragraph, and the follow-up appendix. These don't affect implementability since the operative phase descriptions are correct.

Neither category blocks implementation if the spec is updated as a prerequisite task.

---
*Re-review generated by /review-plan (Pass 3)*
