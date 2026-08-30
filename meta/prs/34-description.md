---
type: "pr-description"
id: "34"
title: "Split the VCS subdomain story into four independent work items"
date: "2026-08-02T18:50:13+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0136", "work-item:0169", "work-item:0185", "work-item:0186", "work-item:0187", "work-item:0188", "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/34"
pr_number: 34
tags: ["work-items", "planning", "vcs", "hooks", "rust", "cli", "migration"]
revision: "3b3ac9a995414cc85b78bda442322ebc3174278f"
repository: "accelerator"
last_updated: "2026-08-02T18:50:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Split the VCS subdomain story into four independent work items

## Summary

0169 had accreted four concerns with very different risk profiles — the VCS subdomain and hooks migration itself, a bash micro-fix to the bootstrap, a build-system generalisation, and the adoption of two new Rust dependency trees. Review 2 returned `REVISE` on exactly that ground, unanimously across the scope, completeness and testability lenses. This PR records the split: 0169 keeps the subdomain and hooks work, three new siblings take the separable concerns, and a fourth takes the cleanup the split defers.

Documentation only — no code, no build-system changes.

## Changes

- **Four new work items.** 0185 (converge `corpus-adapters` on the library-backed VCS adapter), 0186 (remove the exec probe from the bootstrap warm path), 0187 (generalise the sub-binary registration surface), 0188 (library-backed VCS adapter over `gix` and `jj-lib`).
- **0169 substantially rewritten** (679 lines changed) — scope narrowed to the `vcs detect|status|log|guard` subdomain plus the hook migration, with new Terminology, "The guard's inputs", Sequencing Constraints and Validation Results sections. Its `blocked_by` gains 0186, 0187 and 0188; its `blocks` gains 0170, 0171 and 0173.
- **Reciprocal dependency edges recorded up front**, rather than at acceptance, so the coupling is visible from the consuming side while the blocking work is in flight: `blocked_by: 0187` plus a dated Dependencies bullet on 0170–0173, `blocks: 0187` on 0168, and `blocks: 0186` on 0182. 0170's bullet also carries the no-underscore dispatch-token constraint (`work-item`, not `work_item`, because the token derives `ACCELERATOR_<TOKEN>_BIN`).
- **Epic 0136's decomposition annotated** — the four new children placed in their phases, the acceptance criterion widened from "0162–0174" to include 0185–0188, and a note that 0178/0179/0180 are grandchildren via 0166.
- **Research document added** — the codebase research that drove the split.
- **Five review documents recorded** — 0169 reviews 2 (`REVISE`) and 3 (`APPROVE`), and review 1 for each of 0186, 0187 and 0188 (all `APPROVE`).

## Context

- Splits `meta/work/0169-vcs-subdomain-and-hooks-migration.md`, a child of epic 0136 (Migrate Shell Scripts into a Rust CLI).
- Driven by `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`, added here. Its four substantive findings are what forced the split: the `hooks.json` `${CLAUDE_PLUGIN_ROOT}` probe resolved in the story's favour; the PreToolUse envelope the story pinned as its golden fixture turned out to be the deprecated shape; the ~108 ms per-Bash-call cost was localised to `probe_dir` in the bootstrap rather than to the sub-binary choice; and the token `vcs` collides with the existing domain crate in both `tasks/manifest.py` and cargo-pup.
- The `gix`/`jj-lib` feasibility risk was retired empirically in that research (§9) — the combined tree passes `bans`, `advisories` and `sources` against a verbatim copy of `cli/deny.toml`, with `uluru` (MPL-2.0) the sole licence rejection, and a binary calling both cross-compiles to a static musl ELF that `_assert_static_elf` accepts.

## Testing

- [x] `./scripts/validate-corpus-frontmatter.sh meta` — exit 0 across the whole corpus, covering all 18 changed files' frontmatter and typed linkage.
- [x] Reciprocal edges checked by hand in both directions: every `blocked_by` added here has its matching `blocks` on the other item.
- [ ] CI on this PR. No code, build-system or shell files are touched, so no language toolchain check is exercised by this change.

## Notes for Reviewers

**The split boundaries are the thing to review, not the prose.** Each new sibling was carved out because it carries a risk the others do not: 0186 is a bash micro-change to a file 0182 is also editing; 0187 is a build-system generalisation five stories depend on; 0188 lands two dependency trees, a workspace-wide licence exception and a pre-1.0 API bet that should be revertable on its own terms. If any boundary looks wrong, it is cheaper to move it now than after planning starts.

**0188 ships unwired, and that is deliberate.** It adds an adapter without removing `CommandProbe` — no caller reaches the new code until 0169 and 0185 do the consumer work. The cost is that `vcs-adapters` carries two probe implementations until 0185 lands, and the zero-spawn guarantee holds only for the four hook paths in the interim. That trade is recorded in both items.

**One dependency is knowingly left unresolved.** 0186 removes the ~108 ms exec probe but leaves the verify shim's second `sha256_file` (~11.7 ms), because three existing tests in `tests/integration/entrypoint/test_accelerator_entrypoint.py` assert the planted-stub defence it provides. That leaves the warm bootstrap near 41 ms against 0169's `G ≤ 1.1 × B` gate of ≈ 38.6 ms — so 0186 is necessary but may not be sufficient for 0169's latency criterion. Rather than hide it, a dated hand-off note in 0169's Dependencies carries the obligation, and 0169 must either relax the threshold or accept the overrun with a stated rationale before acceptance. No work item owns the residual.

**0182's status changes twice across this stack.** This PR flips it `in-progress` → `ready`; PR #36 flips it `ready` → `done`. Reviewing the two in isolation makes that look like churn — it is the same closeout arriving in two steps.

**Stack position.** This is the base of a three-PR stack: #34 → #35 (jj pin bump) → #36 (close out 0167/0168/0182). Nothing below depends on this PR's content, only on its commits being in the ancestry.
