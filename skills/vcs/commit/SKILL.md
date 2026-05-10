---
name: commit
description: Create VCS commits for session changes. Use when the user wants to
  commit their work with well-structured, atomic commits.
argument-hint: "[optional message or flags]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)
---

# Commit Changes

!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-status.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/vcs-log.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh commit`

## Process:

1. **Think about what changed:**

- Review the conversation history and understand what was accomplished
- Review the VCS status and diff above to see what files changed
- Consider whether changes should be one commit or multiple logical commits

2. **Plan your commit(s):**

- Identify which files belong together
- Draft clear, descriptive commit messages
- Use imperative mood in commit messages
- Focus on why the changes were made, not just what

3. **Present your plan to the user:**

- List the files you plan to include in each commit
- Show the commit message(s) you'll use
- Ask: "I plan to create [N] commit(s) with these changes. Shall I proceed?"

4. **Execute upon confirmation:**

- Stage and commit the planned files with your planned messages
- Use the VCS commands appropriate for this repository (refer to the
  session's VCS context for the correct commands)
- Show recent commit history to confirm the result

## Important:

- **NEVER add co-author information or Claude attribution**
- Commits should be authored solely by the user
- Do not include any "Generated with Claude" messages
- Do not add "Co-Authored-By" lines
- Write commit messages as if the user wrote them
- Use the VCS commands appropriate for this repository's version control
  system (refer to the session's VCS context for the correct commands)
- When staging files, always add specific files by name — never bulk-add
  all changes at once

## Remember:

- You have the full context of what was done in this session
- Group related changes together
- Keep commits focused and atomic when possible
- The user trusts your judgment - they asked you to commit

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh commit`
