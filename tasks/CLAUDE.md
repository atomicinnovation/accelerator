# Build system

- pyrefly and ruff are version-pinned **exactly** in `pyproject.toml` because
  their rule sets are version-sensitive. Shared helpers live in `tasks/shared/`.
- Release/version logic enforces **version coherence**: `plugin.json`, the
  `cli/` workspace `Cargo.toml`, and any version-pinned member manifest must
  agree.
- Registering a new dispatched sub-binary is a thirteen-point surface — see
  `tasks/README.md#registering-a-dispatched-sub-binary` before adding one.
