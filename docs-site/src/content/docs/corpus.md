---
title: Corpus CLI
---

`accelerator corpus` is the sub-binary skills and migrations use to read and
write the `meta/` corpus — ADR numbering, artefact-metadata provenance,
body-section typed-linkage extraction, and frontmatter conformance checking.
It is plumbing rather than a feature you reach for directly: skills invoke it
through the `!`-preprocessor (see [Anatomy of a skill
invocation](internals.md#anatomy-of-a-skill-invocation)), and the `/accelerator:migrate`
framework invokes it to validate a corpus it has just rewritten. Running it
by hand is mainly useful for reproducing what a skill did, or for validating
a corpus outside a skill session.

| Noun group    | Verbs                        | What it does                                                                                               |
|---------------|------------------------------|------------------------------------------------------------------------------------------------------------|
| `adr`         | `next-number`, `read-status` | ADR sequence numbering and reading an ADR's `status:` frontmatter field                                    |
| `metadata`    | `derive`                     | The unified artefact-metadata provenance block (UTC datetime, filename timestamp, VCS repository/revision) |
| `linkage`     | `extract`                    | Typed-linkage records (`parent`, `blocks`, `relates_to`, …) found in a document's body sections            |
| `frontmatter` | `validate`                   | Structural and referential conformance checking against the unified frontmatter schema                     |

See [Internals](internals.md#terminal-invocation) for how to reach
`accelerator` at all from a terminal; everything below assumes that's set up.

## `adr`

```bash
accelerator corpus adr next-number              # the next ADR number
accelerator corpus adr next-number --count 3    # the next three, one per line
accelerator corpus adr read-status meta/decisions/ADR-0042-some-decision.md
```

`next-number` resolves the decisions directory the same way the rest of the
plugin does (`paths.decisions` in `.accelerator/config.md`, falling back to
the catalogue default). `--fail-safe` degrades a failure (a bad config, an
unreadable decisions directory) to an empty, exit-0 result instead of
propagating it — this is what the eager `!`-preprocessor binding in
`create-adr`'s `SKILL.md` uses, so a corpus problem doesn't abort the skill's
preamble.

## `metadata`

```bash
accelerator corpus metadata derive
```

Prints the provenance block every generated artefact's frontmatter carries: a
UTC ISO-8601 datetime, a host-local filename timestamp, and — inside a VCS
checkout — the repository name and current revision. Producer skills
interpolate this block directly into the frontmatter they emit.

## `linkage`

```bash
accelerator corpus linkage extract meta/work/0042-some-work-item.md
```

Scans a document's qualifying body sections (`## Dependencies`,
`## References`, and similar) for typed-linkage mentions and emits one TSV
record per match: `source_type<TAB>key<TAB>target_ref<TAB>anchor<TAB>band`.
The source type is inferred from the file's path unless `--source-type`
overrides it. This is how the `/accelerator:migrate` framework's 0007
migration normalises body-section prose links into typed frontmatter
references.

## `frontmatter`

```bash
accelerator corpus frontmatter validate                        # whole configured corpus
accelerator corpus frontmatter validate --dir meta/work        # one directory
accelerator corpus frontmatter validate --file meta/work/0042-x.md
accelerator corpus frontmatter validate --checks structure     # skip referential checks
```

Checks every in-scope file against the unified schema: required base
fields, quoted `id:`, the provenance bundle's presence or absence by type,
status vocabulary, typed-linkage shape, and (referentially) that every
typed-linkage value resolves to a real artefact somewhere in the corpus.
With no `--dir`/`--file`, it walks every configured doc-type directory.
`--checks` (`structure`, `references`, or both — the default) lets a caller
narrow which category runs; a file made ineligible for the referential check
by a structural failure is reported as `SKIPPED`, not silently folded into a
clean result. Exits non-zero with one `<file>: <CODE> — <message>` line per
violation on stderr.

## Local development

| Mechanism                | Purpose                                                                                                                                         |
|--------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_CORPUS_BIN` | One-shot override pointing `accelerator corpus …` at a locally-built `accelerator-corpus` binary, bypassing the normal fetch-and-cache dispatch |

This mirrors `ACCELERATOR_VISUALISER_BIN` and `ACCELERATOR_VCS_BIN` for the
plugin's other dispatched sub-binaries — set it when working on `cli/corpus/`,
`cli/corpus-adapters/`, or `cli/corpus-cli/` in this repository, so dispatch
resolves the binary you just built instead of trying to fetch a release.
