---
name: create-note
description: Interactively capture a short-form note. Use when jotting down
  an observation, insight, or strategy snippet as a short-form note in
  meta/notes/ — e.g. "make a note of this", "jot this down", "capture a note".
argument-hint: "[note topic]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
---

# Create Note

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-note`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes`

## Note Template

The template below defines the frontmatter every note must carry. Read it now —
use it as the structure you populate in the Write step.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh note`

You are tasked with capturing a short-form note — an observation, insight, or
strategy snippet that does not warrant a research document, plan, or ADR. This
is a lightweight, in-the-moment capture: elicit the note in a single
round-trip, then write it. Do not run multi-agent research or a lengthy
refinement loop.

## Step 0: Parameter Check

When this command is invoked:

1. **If an argument was provided**, treat it as the note's topic. Do not
   re-prompt for a topic — proceed straight to eliciting the note body, any
   optional tags, and an optional related artifact (see Step 1).

2. **If no argument was provided**, respond with:

```
What would you like to note? Give me a short topic and the note itself; tags
and a related work item or plan are optional.
```

Then wait for the user's reply. This greeting **is** the Step 1 elicitation
prompt — do not issue a second prompt before the user responds.

## Step 1: Elicit the Note

Gather, in a single compact exchange:

- **Topic** — a short subject line for the note (skip if supplied as the
  argument).
- **Body** — the note's content: the observation, insight, or snippet.
- **Tags** *(optional)* — clearly marked optional.
- **Related artifact** *(optional)* — a work item or plan the note relates to,
  clearly marked optional.

Aim for one round-trip. Do not interrogate the user across multiple turns.

### Linkage

When the user names a related artifact, record it under `relates_to` by
default. Record it under `parent` **only** when the user confirms that the
artifact *owns* the note. Never infer ownership. Ask a neutral question that
explains the distinction in plain terms and defaults to `relates_to`:

```
Does <artifact> own this note as its parent, or is it just related? [owns / related]
```

Treat any non-affirmative or unclear answer as "related". Never write `source`
or `derived_from` — those keys belong to the artifact extracted *from* a note,
not to the note itself.

## Step 2: Derive the Filename

1. Run `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh` to obtain
   the current date/time, revision, and repository name. Run the bare path
   **directly** as an executable; never prefix it with `bash`/`sh`/`env` (a wrapper
   prefix escapes the skill's `allowed-tools` permission and forces an unnecessary
   prompt).
2. Build the path `<notes_dir>/YYYY-MM-DD-<topic-slug>.md`, where the date is
   the `Current Date/Time (UTC):` date portion and `<topic-slug>` is a
   meaningful kebab-case summary of the topic — condense and normalise it,
   never a raw passthrough of the input, and never empty.
3. **Collision handling (auto-disambiguate)**: if that path already exists (a
   same-day, same-topic note), probe `<slug>-2.md`, then `<slug>-3.md`, … and
   write to the first index that does not yet exist. Do not overwrite and do
   not abort — the just-elicited note must be preserved.

## Step 3: Populate frontmatter

Substitute every field below into the template's frontmatter block, using the
helper output from Step 2:

- `type:` ← `note`
- `id:` ← the final filename stem (the path from Step 2 without `.md`), quoted
  as a YAML string
- `title:` ← a concise title for the note
- `date:` ← the `Current Date/Time (UTC):` value
- `author:` ← the author resolved per the standard chain (config → VCS user →
  prompt)
- `producer:` ← `create-note`
- `status:` ← `captured`
- `topic:` ← the note's topic
- `tags:` ← the supplied tags as a YAML array (`[]` when none)
- `revision:` ← the `Current Revision:` value
- `repository:` ← the `Repository Name:` value
- `last_updated:` ← the same `Current Date/Time (UTC):` value
- `last_updated_by:` ← the same value resolved for `author`
- `schema_version:` ← `1` (bare integer)

The typed-linkage keys are omit-when-empty: write each only when it has a
value, and omit the key entirely otherwise.

- `parent:` ← the owning artifact as a typed-linkage ref
  (`"work-item:NNNN"`); fill only when the user confirms that artifact
  owns the note, otherwise omit the key.
- `relates_to:` ← list of typed-linkage refs to related artifacts
  (`["work-item:NNNN", ...]`); fill when the user names a related
  artifact, otherwise omit the key.

This skill never writes `source` or `derived_from`: those keys are owned by the
artifact extracted from a note, not by the note.

## Step 4: Write the Note

1. Create the notes directory if it does not exist.
2. Write the file to the path derived in Step 2, with the substituted
   frontmatter and the note body beneath the H1 (`# <title>`).
3. Print a confirmation naming the literal path written:

```
Note created: <notes_dir>/YYYY-MM-DD-<slug>.md
```

   On a disambiguated write, show the qualified slug and point at the existing
   note so both files are actionable:

```
Note created: <final-path> (an earlier note on this topic exists at <first-path>; this one was written as <slug>-N.md)
```

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-note`
