---
type: "note"
id: "2026-10-23-guarded-working-copy-configured-paths"
title: "Derive guarded working-copy scopes from configured paths"
date: "2026-10-23T12:12:19+00:00"
author: "Toby Clemson"
producer: "create-note"
status: "captured"
topic: "Make cli/vcs/src/guarded_working_copy use the configured paths rather than hardcoding /meta"
tags: []
revision: "0e95a54d85c430c45a2835fe8d33e4c90b53c442"
repository: "accelerator"
last_updated: "2026-10-23T12:12:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Derive guarded working-copy scopes from configured paths

`GUARDED_SCOPES` in `cli/vcs/src/guarded_working_copy.rs` hardcodes `meta/`
alongside `.claude/accelerator` and `.accelerator/`. A project that relocates
its accelerator documents through configuration (for example a non-default
work, notes, or plans directory) is therefore not guarded: a dirty file under
the configured location does not block a state-rewriting command such as
`work rekey`, while an unrelated `meta/` file does.

The scopes should be derived from the resolved document paths in the project's
configuration rather than from a constant. `observe_guarded` already takes
`scopes` as a parameter, so the change sits mostly with the callers:

- `cli/work-cli/src/rekey.rs` passes `GUARDED_SCOPES` directly.
- `cli/work-adapters/tests/rekey_working_copy.rs`,
  `cli/work-adapters/tests/support/mod.rs` and
  `cli/migrate-adapters/tests/working_copy.rs` pin the constant.
- `cli/vcs/tests/fixtures/public-api.txt` records it as public API, so
  removing or reshaping it is a public-API change.

The `.claude/accelerator` and `.accelerator/` config scopes remain fixed; only
the document-root scope needs to come from configuration.
