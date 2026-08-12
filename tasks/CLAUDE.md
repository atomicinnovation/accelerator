# Build system

- pyrefly and ruff are version-pinned **exactly** in `pyproject.toml` because
  their rule sets are version-sensitive. Shared helpers live in `tasks/shared/`.
- Release/version logic enforces **version coherence**: `plugin.json`, the
  `cli/` workspace `Cargo.toml`, and any version-pinned member manifest must
  agree. `cli/Cargo.lock` counts too — it carries a copy of the version per
  workspace member, so `version.write` syncs it via `cargo metadata` (the
  minimal update, never `generate-lockfile`). Clippy runs `--locked`, so drift
  there surfaces as an unrelated-looking Rust failure;
  `tests/unit/tasks/test_version.py` guards it by name instead.
- Registering a new dispatched sub-binary is a thirteen-point surface — see
  `tasks/README.md#registering-a-dispatched-sub-binary` before adding one.
  Registering a plain library crate is a smaller surface — see
  `tasks/README.md#registering-a-library-crate`.
