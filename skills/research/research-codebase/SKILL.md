---
name: research-codebase
description: Conduct comprehensive codebase research by spawning parallel
  subagents and synthesising findings into a research document. Use when the
  user needs to deeply understand a codebase area or answer technical questions.
argument-hint: "[research question]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
---

# Research Codebase

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh research-codebase`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research_codebase`
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions`

You are tasked with conducting comprehensive research across the codebase to
answer user questions by spawning parallel sub-agents and synthesising their
findings.

## Initial Setup:

When this command is invoked, respond with:

```
I'm ready to research the codebase. Please provide your research question or area of interest, and I'll analyse it thoroughly by exploring relevant components and connections.
```

Then wait for the user's research query.

## Steps to follow after receiving the research query:

### Step 1: Read any directly mentioned files first

- If the user mentions specific files (work items, docs, JSON), read them FULLY
  first
- **IMPORTANT**: Use the Read tool WITHOUT limit/offset parameters to read
  entire files
- **CRITICAL**: Read these files yourself in the main context before spawning
  any sub-tasks
- This ensures you have full context before decomposing the research

### Step 2: Analyse and decompose the research question

- Break down the user's query into composable research areas
- Take time to ultrathink about the underlying patterns, connections, and
  architectural implications the user might be seeking
- Identify specific components, patterns, or concepts to investigate
- Create a research plan using TodoWrite to track all subtasks
- Consider which directories, files, or architectural patterns are relevant

### Step 3: Spawn parallel sub-agent tasks for comprehensive research

- Create multiple Task agents to research different aspects concurrently
- We now have specialised agents that know how to do specific research tasks:

**For codebase research:**

- Use the **{codebase locator agent}** agent to find WHERE files and components live
- Use the **{codebase analyser agent}** agent to understand HOW specific code works
- Use the **{codebase pattern finder agent}** agent if you need examples of similar
  implementations

**For meta directory:**

- Use the **{documents locator agent}** agent to discover what documents exist about the
  topic in the configured research, plans, and decisions directories (shown
  above)
- Use the **{documents analyser agent}** agent to extract key insights from specific
  documents (only the most relevant ones)

**For web research (only if user explicitly asks):**

- Use the **{web search researcher agent}** agent for external documentation and
  resources
- IF you use web-research agents, instruct them to return LINKS with their
  findings, and please INCLUDE those links in your final report

The key is to use these agents intelligently:

- Start with locator agents to find what exists
- Then use analyser agents on the most promising findings
- Run multiple agents in parallel when they're searching for different things
- Each agent knows its job - just tell it what you're looking for
- Don't write detailed prompts about HOW to search - the agents already know

### Step 4: Wait for all sub-agents to complete and synthesise findings

- IMPORTANT: Wait for ALL sub-agent tasks to complete before proceeding
- Compile all sub-agent results (both codebase and document findings)
- Prioritise live codebase findings as primary source of truth
- Use document findings as supplementary historical context
- Connect findings across different components
- Include specific file paths and line numbers for reference
- Verify all output paths are correct
- Highlight patterns, connections, and architectural decisions
- Answer the user's specific questions with concrete evidence

### Step 5: Gather metadata for the research document

- Run the `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
  script to generate all relevant metadata. Run the bare path **directly** as an
  executable; never prefix it with `bash`/`sh`/`env` (a wrapper prefix escapes the
  skill's `allowed-tools` permission and forces an unnecessary prompt).
- Filename: write to the configured research directory (shown above) using
  - Format: `YYYY-MM-DD-ENG-XXXX-description.md` where:
    - YYYY-MM-DD is today's date
    - ENG-XXXX is the work item number (omit if no work item)
    - description is a brief kebab-case description of the research topic
  - Examples:
    - With work item: `2025-01-08-ENG-1478-parent-child-tracking.md`
    - Without work item: `2025-01-08-authentication-flow.md`

### Step 6: Populate frontmatter and generate the research document

- Use the metadata gathered in step 5
- Structure the document with YAML frontmatter followed by content using
  this template:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh codebase-research`

  Before writing the artifact file, **substitute** every field below
  with the indicated value, using the helper output captured in
  step 5:
  - `type:` ← `codebase-research`
  - `id:` ← the filename stem (the file path computed above without
    `.md`), always quoted as a YAML string
  - `title:` ← `Research: {User's Question/Topic}`
  - `date:` ← the `Current Date/Time (UTC):` value
  - `author:` ← the author resolved per the standard chain
    (config → VCS user → prompt)
  - `producer:` ← `research-codebase`
  - `status:` ← `complete`
  - `revision:` ← the `Current Revision:` value
  - `repository:` ← the `Repository Name:` value
  - `last_updated:` ← the same `Current Date/Time (UTC):` value
  - `last_updated_by:` ← the same value resolved for `author`
  - `schema_version:` ← `1` (bare integer)

  Optional linkage/foreign-ref keys are omit-by-default: the
  template shows each as `""`/`[]`, but write a key into the artifact
  **only** when it has a value, and omit it entirely otherwise (do not
  carry the empty placeholder through).

  - `parent:` ← the work item this research supports, as a typed-linkage
    ref (`"work-item:NNNN"`). Fill when the research has an owning work
    item; otherwise omit the key.
  - `relates_to:` ← list of typed-linkage refs to related research
    (`["codebase-research:NNNN", ...]`). Fill when related artifacts are
    explicit; otherwise omit the key.
  - `work_item_id:` ← the linked work item's full ID (quoted). Fill when
    the research is linked to a work item; otherwise omit the key.

### Step 7: Add GitHub permalinks (if applicable)

- Check if on main branch or if commit is pushed: `git branch --show-current`
  and `git status`
- If on main/master or pushed, generate GitHub permalinks:
  - Get repo info: `gh repo view --json owner,name`
  - Create permalinks:
    `https://github.com/{owner}/{repo}/blob/{commit}/{file}#L{line}`
- Replace local file references with permalinks in the document

### Step 8: Present findings

- Present a concise summary of findings to the user
- Include key file references for easy navigation
- Ask if they have follow-up questions or need clarification

### Step 9: Handle follow-up questions

- If the user has follow-up questions, append to the same research document
- Update the frontmatter fields `last_updated` and `last_updated_by` to reflect
  the update
- Add `last_updated_note: "Added follow-up research for [brief description]"` to
  frontmatter
- Add a new section: `## Follow-up Research [timestamp]`
- Spawn new sub-agents as needed for additional investigation
- Continue updating the document and syncing

## Important notes:

- Always use parallel Task agents to maximise efficiency and minimise context
  usage
- Always run fresh codebase research - never rely solely on existing research
  documents
- The configured document directories provide historical context to supplement
  live findings
- Focus on finding concrete file paths and line numbers for developer reference
- Research documents should be self-contained with all necessary context
- Each sub-agent prompt should be specific and focused on read-only operations
- Consider cross-component connections and architectural patterns
- Include temporal context (when the research was conducted)
- Link to GitHub when possible for permanent references
- Keep the main agent focused on synthesis, not deep file reading
- Encourage sub-agents to find examples and usage patterns, not just definitions
- Explore all configured document directories (research, plans, decisions)
- **File reading**: Always read mentioned files FULLY (no limit/offset) before
  spawning sub-tasks
- **Critical ordering**: Follow the numbered steps exactly
  - ALWAYS read mentioned files first before spawning sub-tasks (step 1)
  - ALWAYS wait for all sub-agents to complete before synthesising (step 4)
  - ALWAYS gather metadata before writing the document (step 5 before step 6)
  - NEVER write the research document with placeholder values
- **Frontmatter consistency**:
  - Always include frontmatter at the beginning of research documents
  - Keep frontmatter fields consistent across all research documents
  - Update frontmatter when adding follow-up research
  - Use snake_case for multi-word field names (e.g., `last_updated`,
    `git_commit`)
  - Tags should be relevant to the research topic and components studied

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh research-codebase`
