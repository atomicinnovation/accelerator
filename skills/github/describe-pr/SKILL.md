---
name: describe-pr
description: Generate a comprehensive pull request description following the
  repository's standard template. Use when the user wants to create or update a
  PR description.
argument-hint: "[PR number or URL]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/github/scripts/*)
---

# Generate PR Description

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh describe-pr`

**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp`

**IMPORTANT**: Wherever `{prs directory}` or `{tmp directory}` appears in
the instructions below, substitute the actual resolved path shown above.

**PR description template**:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh pr-description`

You are tasked with generating a comprehensive pull request description
following the repository's standard template.

## Steps to follow:

### Step 1: Use the PR description template

- The template is shown above under "PR description template"
- Read the template carefully to understand all sections and requirements

### Step 2: Identify the PR to describe

- Check if the current branch has an associated PR:
  `gh pr view --json url,number,title,state 2>/dev/null`
- If no PR exists for the current branch, or if on main/master, list open PRs:
  `gh pr list --limit 10 --json number,title,headRefName,author`
- Ask the user which PR they want to describe

### Step 3: Check for existing description

- Check if `{prs directory}/{number}-description.md` already exists
- If it exists, read it and inform the user you'll be updating it
- Consider what has changed since the last description was written

### Step 4: Gather comprehensive PR information

- Get the full PR diff: `gh pr diff {number}`
- If you get an error about no default remote repository, instruct the user to
  run `gh repo set-default` and select the appropriate repository
- Get commit history: `gh pr view {number} --json commits`
- Review the base branch: `gh pr view {number} --json baseRefName`
- Get PR metadata: `gh pr view {number} --json url,title,number,state`

### Step 5: Analyze the changes thoroughly

(ultrathink about the code changes, their architectural implications, and
potential impacts)

- Read through the entire diff carefully
- For context, read any files that are referenced but not shown in the diff
- Understand the purpose and impact of each change
- Identify user-facing changes vs internal implementation details
- Look for breaking changes or migration requirements

### Step 6: Handle verification requirements

- Look for any checklist items in the "How to verify it" section of the template
- For each verification step:
  - If it's a command you can run (like `make check test`, `npm test`, etc.),
    run it
  - If it passes, mark the checkbox as checked: `- [x]`
  - If it fails, keep it unchecked and note what failed: `- [ ]` with
    explanation
  - If it requires manual testing (UI interactions, external services), leave
    unchecked and note for user
- Document any verification steps you couldn't complete

### Step 7: Generate the description

- Fill out each section from the template thoroughly:
  - Answer each question/section based on your analysis
  - Be specific about problems solved and changes made
  - Focus on user impact where relevant
  - Include technical details in appropriate sections
  - Write a concise changelog entry
- Ensure all checklist items are addressed (checked or explained)

### Step 8: Populate frontmatter and save the description

  Use the unified pr-description template as the source of the
  frontmatter block:

  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh pr-description`

  Before writing the artifact file, capture metadata and substitute
  the unified base fields into the template's frontmatter block:

  1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
     to obtain `Current Date/Time (UTC):`, `Current Revision:`, and
     `Repository Name:`. Run the bare path **directly** as an executable;
     never prefix it with `bash`/`sh`/`env` (a wrapper prefix escapes the
     skill's `allowed-tools` permission and forces an unnecessary prompt). Also
     capture PR-specific extras via
     `gh pr view <number> --json url,number,title,mergeCommit` to
     fill `pr_url:`, `pr_number:`, `title:`, and (when the PR is
     merged) `merge_commit:`.
  2. **Substitute** every field below with the indicated value:
     - `type:` ← `pr-description`
     - `id:` ← the PR number as a quoted YAML string (e.g. `"42"`)
     - `title:` ← the PR title from `gh pr view`
     - `date:` ← the `Current Date/Time (UTC):` value
     - `author:` ← the author resolved per the standard chain
       (config → VCS user → prompt)
     - `producer:` ← `describe-pr`
     - `status:` ← `complete`
     - `pr_url:` ← the URL from `gh pr view`
     - `pr_number:` ← the PR number as a bare integer
     - `merge_commit:` ← the merge commit SHA. Fill when the PR is
       merged; otherwise omit the key entirely.
     - `revision:` ← the `Current Revision:` value
     - `repository:` ← the `Repository Name:` value
     - `last_updated:` ← the same `Current Date/Time (UTC):` value
     - `last_updated_by:` ← the same value resolved for `author`
     - `schema_version:` ← `1` (bare integer)

     Optional linkage/foreign-ref keys are omit-by-default:
     the template shows each as `""`/`[]`, but write a key into the
     artifact **only** when it has a value, and omit it entirely
     otherwise (do not carry the empty placeholder through).

     - `parent:` ← the work item this PR implements, as a typed-linkage
       ref (`"work-item:NNNN"`). Fill when the PR has an owning work
       item; otherwise omit the key.
     - `relates_to:` ← list of typed-linkage refs to related artifacts
       (`["work-item:NNNN", ...]`). Fill when relationships are explicit;
       otherwise omit the key.
     - `work_item_id:` ← the linked work item's full ID (quoted). Fill
       when the PR is linked to a work item; otherwise omit the key.
  3. Write the completed description with the substituted frontmatter
     block to `{prs directory}/{number}-description.md`.

- Show the user the generated description (without frontmatter — they'll
  see what gets posted to GitHub).
- On re-run (when `{prs directory}/{number}-description.md` already
  exists), regenerate the unified frontmatter rather than only updating
  `date:`. Creation-time fields are immutable on re-run: `date:`,
  `author:`, `id:`, `pr_number:`, and `pr_url:` are preserved verbatim
  from the existing on-disk file. `last_updated:` is refreshed to the
  new `Current Date/Time (UTC):` value; `last_updated_by:` is
  rewritten to the current author per the standard resolution chain;
  `merge_commit:` is filled if the PR is now merged. Step 3 already
  handles reading the prior description for context.

### Step 9: Update the PR

- The `{prs directory}/{number}-description.md` file contains YAML frontmatter
  that should not appear on GitHub. Before posting, strip the frontmatter
  block from the start of the file:
  1. Read the file content
  2. The frontmatter block starts with `---` on line 1 and ends at the
     next `---` line (which closes the YAML block). Only match the
     opening frontmatter block — do not match `---` lines that appear
     later in the body (e.g., markdown horizontal rules).
  3. Ensure the tmp directory exists: `mkdir -p {tmp directory}`
  4. Write everything after the closing `---` line to
     `{tmp directory}/pr-body-{number}.md`
  5. Post the body via the helper script, which resolves the base
     (upstream) repository for cross-fork safety and PATCHes via the
     GitHub REST API:
     `${CLAUDE_PLUGIN_ROOT}/skills/github/describe-pr/scripts/pr-update-body.sh {number} {tmp directory}/pr-body-{number}.md`
     If the helper exits non-zero, surface its stderr verbatim to the
     user — it includes preserved `gh` error text and, where applicable,
     a `gh repo set-default` remediation hint. Exit codes:
     - **Exit 1** → encode failed (`pr-update-body.sh:` stderr prefix)
       OR resolver-resolution failed (`pr-base-repo.sh:` prefix)
     - **Exit 2** → usage error / missing jq
     - **Exit 4** → PATCH failed
     The stderr prefix identifies which stage failed when exit code
     alone is ambiguous.
  6. Clean up `{tmp directory}/pr-body-{number}.md`
- Confirm the update was successful
- If any verification steps remain unchecked, remind the user to complete
  them before merging

## Important notes:

- This command works across different repositories - always read the local
  template
- Be thorough but concise - descriptions should be scannable
- Focus on the "why" as much as the "what"
- Include any breaking changes or migration notes prominently
- If the PR touches multiple components, organize the description accordingly
- Always attempt to run verification commands when possible
- Clearly communicate which verification steps need manual testing

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh describe-pr`
