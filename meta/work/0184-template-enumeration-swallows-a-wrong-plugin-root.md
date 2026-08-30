---
type: "work-item"
id: "0184"
title: "template_names succeeds-with-nothing on a plugin root that is not an installation"
date: "2026-07-29T00:00:00+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "draft"
kind: "bug"
priority: "low"
relates_to: ["work-item:0182"]
tags: ["bug", "cli", "config", "templates", "plugin-root"]
last_updated: "2026-07-29T00:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-714"
---

# 0184: template_names succeeds-with-nothing on a plugin root that is not an installation

**Kind**: Bug
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

`FileConfigStore::template_names`
([`cli/config-adapters/src/store.rs`](../../cli/config-adapters/src/store.rs))
now refuses when the plugin root is *unknown*, but still succeeds with an empty
list when the root is *known and wrong*:

```rust
let Ok(entries) = fs::read_dir(plugin.join("templates")) else {
    return Ok(Vec::new());
};
```

That arm swallows every `read_dir` failure alike — a root pointing at a
directory that is not an Accelerator installation, a root pointing at a deleted
path, and a genuine permission or I/O error. The observable is a header-only
`config templates list` table at exit 0: the same silent-wrong-answer mode 0182
closed for the unknown-root case, surviving one step further out.

## Context

0182 Phase 5 converted a missing root into `ConfigError::PluginRootUnavailable`
at the three plugin-content consumers, and deliberately left this arm alone so
the phase stayed scoped to the root's *presence*. The surviving behaviour is
pinned as a characterisation test —
`a_root_without_a_templates_directory_still_renders_an_empty_table` in
`cli/launcher/tests/config_read.rs` — so it is visible rather than implicit, and
that test is what this item would change.

A wrong root became newly plausible in the same change: the bootstrap derives
the root from its own location, and a directly-invoked launcher passes no
`plugin.json` gate that would catch a root pointing somewhere unexpected.

## Requirements

- Distinguish the `read_dir` failure kinds instead of collapsing them:
  `ErrorKind::NotFound` (the root carries no `templates/` directory, so it is
  not an installation) becomes `ConfigError::PluginRootUnavailable`; every other
  error becomes `ConfigError::Io` naming the path, matching the treatment the
  other enumerators in this file already give (`custom_lenses`, `skill_names`).
- Decide whether the same reasoning applies to `resolve_template`'s
  plugin-default tier and to `plugin_template_path`, which today test
  `is_file()` and fall through — a wrong root there yields "not found" for every
  template name, which is a better message than an empty table but still names
  the template rather than the root.
- Keep the root-independent families and project-local overrides working: a
  wrong root must not break `config template <name>` when a user override
  resolves, which is the property
  `a_user_override_still_resolves_with_no_plugin_root` pins for the absent-root
  case.

## Acceptance Criteria

- [ ] `config templates list` against a root that exists but carries no
      `templates/` directory exits non-zero with a diagnostic naming
      `ACCELERATOR_PLUGIN_ROOT`, replacing the header-only table at exit 0.
- [ ] A genuine I/O failure on a present `templates/` directory (e.g. mode
      `0o000`) yields a `ConfigError::Io` naming the path, not the plugin-root
      refusal — the two are distinguishable from the message alone.
- [ ] The characterisation test named above is replaced by its inverse rather
      than deleted, so the change of behaviour is recorded at the same site.
- [ ] A user override still resolves against a wrong root.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Blocked by: 0182 (introduces `PluginRootUnavailable` and the two named
  accessors this builds on).

## Assumptions

- Every Accelerator installation ships a `templates/` directory, so its absence
  is a reliable signal that the root is not an installation. Worth confirming
  against the release artifact's file list before relying on it.

## References

- `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
- `meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md`
  — Phase 5 §3, "What this does and does not buy"
