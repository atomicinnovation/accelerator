# Hooks

- **Hooks keep the pathed launcher form on purpose — do not convert them to
  bare `accelerator`.** `hooks.json` dispatches through
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator …`, and `launcher-link-refresh.sh`
  self-locates its own launcher by absolute path. Skills converged onto bare
  `accelerator …` under work item 0245, but hooks are deliberately out of that
  scope.
- **Why the two surfaces differ.** A hook receives `${CLAUDE_PLUGIN_ROOT}` as a
  real exported environment variable, so the pathed form always resolves. Skills
  rely instead on Claude Code placing the plugin `bin/` on the `!`-preprocessor
  / Bash-tool `PATH`, where bare `accelerator` resolves to the same
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`. A hook has no equivalent guarantee
  that the plugin `bin/` is on its `PATH`, so bare `accelerator` can fail to
  resolve in a hook. (`launcher-link-refresh.sh` maintains a stable
  `${CLAUDE_PLUGIN_DATA}/bin/accelerator` pointer for the separate terminal-
  invocation convenience — a developer symlinks their own `PATH` at it; that is
  not a hook's resolution path.)
- **Nothing else guards this.** The bare-invocation lint
  (`tasks/lint/bare_invocation.py`) scans only `skills/**/SKILL.md`, so it will
  not stop a contributor from "unifying" hooks onto the bare form and breaking
  dispatch. This file is the standing rationale, because `hooks.json` is strict
  JSON and cannot carry a comment.
