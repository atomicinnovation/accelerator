# How It Works

## Philosophy

Accelerator structures development into discrete phases — research, plan,
implement — where each phase runs with minimal context and communicates with
the next through the filesystem. This design is intentional: by writing
research findings, plans, and other artifacts to disk rather than holding them
in the conversation, each step stays focused and avoids the quality degradation
that comes with large, cluttered context windows.

The result is a development workflow where:

- Each phase has a clear purpose and bounded scope
- The filesystem (specifically the `meta/` directory) serves as persistent
  shared memory between phases and sessions
- Subagents handle exploratory work in isolation, returning only summaries to
  the main context
- Human review happens at the highest-leverage points (research quality and
  plan quality) before implementation begins

For example, a research phase might read 50 files across a codebase, but only
a structured summary is written to disk and passed to the planning phase —
keeping the planner focused and accurate.

## VCS Detection

Accelerator automatically detects whether a repository uses git or
[jujutsu (jj)](https://github.com/jj-vcs/jj) and adapts its behaviour
accordingly. A `SessionStart` hook inspects the working directory for `.jj/` and
`.git/` directories, injecting VCS-specific context (command references and
conventions) into the session. Detection also recognises git **linked
worktrees** — where `.git` is a file (a `gitdir:` pointer) rather than a
directory — so worktree-based sessions are detected just like plain checkouts. A
complementary `PreToolUse` guard warns when raw git commands are used in a
jujutsu repository.

This means all VCS-aware skills — `commit`, `respond-to-pr`, and ad-hoc
interactions — use the correct CLI commands without manual configuration. The
detection covers three modes:

| Mode               | Detected when      | VCS commands used |
|--------------------|--------------------|-------------------|
| **git**            | `.git/` only       | `git`             |
| **jj (colocated)** | `.jj/` and `.git/` | `jj`              |
| **jj (pure)**      | `.jj/` only        | `jj`              |
