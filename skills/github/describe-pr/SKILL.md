---
name: describe-pr
description: Generate a comprehensive pull request description following the
  repository's standard template. Use when the user wants to create or update a
  PR description.
argument-hint: "[PR number or URL]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

# Generate PR Description

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`

**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs meta/prs`

**PR description template**:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh pr-description`

You are tasked with generating a comprehensive pull request description
following the repository's standard template.

## Steps to follow:

1. **Use the PR description template:**

- The template is shown above under "PR description template"
- Read the template carefully to understand all sections and requirements

2. **Identify the PR to describe:**

- Check if the current branch has an associated PR:
  `gh pr view --json url,number,title,state 2>/dev/null`
- If no PR exists for the current branch, or if on main/master, list open PRs:
  `gh pr list --limit 10 --json number,title,headRefName,author`
- Ask the user which PR they want to describe

3. **Check for existing description:**

- Check if `{prs directory}/{number}-description.md` already exists
- If it exists, read it and inform the user you'll be updating it
- Consider what has changed since the last description was written

4. **Gather comprehensive PR information:**

- Get the full PR diff: `gh pr diff {number}`
- If you get an error about no default remote repository, instruct the user to
  run `gh repo set-default` and select the appropriate repository
- Get commit history: `gh pr view {number} --json commits`
- Review the base branch: `gh pr view {number} --json baseRefName`
- Get PR metadata: `gh pr view {number} --json url,title,number,state`

5. **Analyze the changes thoroughly:** (ultrathink about the code changes, their
   architectural implications, and potential impacts)

- Read through the entire diff carefully
- For context, read any files that are referenced but not shown in the diff
- Understand the purpose and impact of each change
- Identify user-facing changes vs internal implementation details
- Look for breaking changes or migration requirements

6. **Handle verification requirements:**

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

7. **Generate the description:**

- Fill out each section from the template thoroughly:
  - Answer each question/section based on your analysis
  - Be specific about problems solved and changes made
  - Focus on user impact where relevant
  - Include technical details in appropriate sections
  - Write a concise changelog entry
- Ensure all checklist items are addressed (checked or explained)

8. **Save and show the description:**

- Write the completed description to `{prs directory}/{number}-description.md`
  with YAML frontmatter:

  ```markdown
  ---
  date: "{ISO timestamp}"
  type: pr-description
  skill: describe-pr
  pr_number: {number}
  pr_title: "{title}"
  status: complete
  ---

  {The generated PR description}
  ```

- Show the user the generated description (without frontmatter — they'll
  see what gets posted to GitHub)
- On re-run (when `{prs directory}/{number}-description.md` already exists),
  regenerate the frontmatter with an updated `date` timestamp. The
  existing step 3 already handles reading the prior description for
  context; the frontmatter is simply regenerated fresh.

9. **Update the PR:**

- The `{prs directory}/{number}-description.md` file contains YAML frontmatter
  that should not appear on GitHub. Before posting, strip the frontmatter
  block from the start of the file:
  1. Read the file content
  2. The frontmatter block starts with `---` on line 1 and ends at the
     next `---` line (which closes the YAML block). Only match the
     opening frontmatter block — do not match `---` lines that appear
     later in the body (e.g., markdown horizontal rules).
  3. Write everything after the closing `---` line to a temporary file
  4. Post with `gh pr edit {number} --body-file /tmp/pr-body-{number}.md`
  5. Clean up the temporary file
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
