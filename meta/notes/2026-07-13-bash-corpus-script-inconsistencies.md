---
type: note
id: "2026-07-13-bash-corpus-script-inconsistencies"
title: "Inconsistencies in the bash corpus scripts surfaced by the 0179 port"
date: "2026-07-13T10:16:57+00:00"
author: "Toby Clemson"
producer: create-note
status: captured
relates_to: ["work-item:0179"]
topic: "Bash corpus script inconsistencies surfaced by the corpus crate port"
tags: ["corpus", "linkage", "doc-types", "tech-debt", "rust-migration"]
revision: "84a6b82ae0ee9964a036d54cd5e7a00db01bad11"
repository: "accelerator"
last_updated: "2026-07-13T10:16:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Inconsistencies in the bash corpus scripts surfaced by the 0179 port

Porting the corpus conventions into the `corpus` / `corpus-adapters` crates
meant reading several bash surfaces that had each grown their own copy of the
same facts. Where they disagreed, the port had to pick a side. This note records
every disagreement found, what the crates chose, and what is left to reconcile.

Everything below was verified by execution against the live scripts, not read
off the source.

## 1. Three encodings of the directory-to-type fact, and one is the odd one out

The mapping from a directory to a doc-type name is written down three times:

| Surface | Source of the table | Match rule |
| --- | --- | --- |
| `scripts/doc-type-inference.sh` (`infer_type_from_path`) | injected from config | longest configured dir wins |
| `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk` (`path_to_typed`) | injected from config (`-v doc_type_table`) | longest configured dir wins |
| `scripts/linkage-parser.sh` (`lp_type_from_path`) | **hardcoded glob arms** | **first arm wins**, ordered by hand |

The first two are config-driven and deliberately kept aligned — `doc-type-inference.sh`
says so in its header, and `test-migrate-0007.sh` asserts it. `lp_type_from_path`
was never brought into that alignment: it is a hand-ordered `case` over literal
path globs, with a comment explaining that the review arms must precede the
generic `*/work/*` and `*/plans/*` arms or they would be shadowed. That ordering
hack is only necessary *because* it is first-match rather than longest-match.

The practical consequence: **linkage is not config-aware.** A repository that
re-paths its corpus (`meta/work` → something else) gets correct validation and
correct migration, but linkage silently stops resolving — references degrade to
raw paths in the `ambiguous` band. Nothing detects this.

The crates resolve this by making linkage table-driven: `corpus::linkage` takes
the same injected doc-type table the other surfaces take, and derives
path-to-type through `corpus::doc_type::infer` (longest-match). One encoding, in
`DocTypeKey`.

## 2. `lp_type_from_path` has no `meta/prs` arm

Falling out of the above: `lp_type_from_path` has arms for `*/reviews/prs/*`
(pr-review) but none for `*/prs/*` (pr-description). So a path reference to a PR
description is unresolvable in bash linkage.

Verified — bash, given a work item whose references list
`` `meta/prs/240-ship-it.md` ``, emits:

```
work-item	relates_to	meta/prs/240-ship-it.md	body:references#0	ambiguous
```

The raw path is carried through as the `target_ref`. The crate, being
table-driven, resolves the same token to `pr-description:240-ship-it`.

Both land in the `ambiguous` band — `pr-description` is not a `target_type` in
`linkage-type-pairs.tsv`, so neither can promote it to `resolved` — but the
emitted `target_ref` differs. This is a real behavioural divergence between the
bash oracle and the crate that the parity fixtures do not cover (no fixture
carries a `meta/prs` path token), and it is the one place the port knowingly
does not reproduce bash.

The crate's behaviour matches what the migration awk already does (the
`doc-type-inference.sh` header explicitly notes a fixture asserting a `meta/prs`
path resolves to `pr-description`), so bash linkage looks like the outlier
rather than the intent.

**Open question**: should `pr-description` be a legal linkage `target_type`? It
is absent from `linkage-type-pairs.tsv`, which is why the reference stays
ambiguous in both implementations. If PR descriptions are meant to be linkable
targets, the pairs table needs a row; if they are not, the migration awk should
arguably not be typifying them either.

## 3. The design-inventory id: parent directory, or the basename `inventory`? **[FIXED]**

Design inventories are nested manifests — `<dir>/<slug>/inventory.md`, where the
manifest basename is always `inventory`. Three surfaces derived the id; two
agreed and one did not:

- `scripts/linkage-parser.sh` derives it from the **parent directory**, with an
  explicit comment: "the id is the parent directory name (matching the
  migration's derive_stem), not the manifest basename `inventory`".
- The 0007 migration's own shell-side `derive_stem` **also** takes the parent
  directory, with a comment saying "so distinct inventories don't all collapse
  to the id `inventory`".
- `0007-frontmatter-rewrite.awk`'s `path_to_typed` had **no nested-manifest
  arm**. It fell through to the whole-stem default and yielded
  `design-inventory:inventory`.

The awk is the surface that rewrites *references* to an inventory inside another
document's linkage, while `derive_stem` sets the inventory's **own** id. So the
migration derived each inventory's id correctly from its directory, and then
wrote every reference to an inventory as `design-inventory:inventory` — pointing
at an id no document has. Every inventory reference in a migrated repository
collapsed onto the same dangling target.

`test-migrate-0007.sh` passed its full suite without catching it: the
`path_to_typed` probe covered the work-item, ADR, PR and research arms, but no
design-inventory path.

**Fixed** — `path_to_typed` now takes the parent directory for
`design-inventory`, matching `derive_stem` and `linkage-parser.sh`. The probe in
`test-migrate-0007.sh` gained two nested inventories under different dated
directories, so an id derived from the basename would collapse them together and
fail. The crate's single-source suite no longer excludes the arm: all 13
configured directories now assert `type:id` agreement between the awk and
`corpus`.

## 4. Four names for every doc type

Each doc type carries four distinct names across the codebase, and which one is
correct depends entirely on which surface you are speaking to:

| Purpose | Example | Now on `DocTypeKey` |
| --- | --- | --- |
| Visualiser wire token | `work-items` | `wire_str()` |
| Config path key | `work` | `config_path_key()` |
| Typed-linkage vocabulary | `work-item` | `linkage_type_name()` |
| Human label | `Work items` | `label()` |

These are all now single-sourced on `DocTypeKey`, so the crate side is
consistent — but the *existence* of four vocabularies for one concept is itself
the smell. `plans` / `plans` / `plan` / `Plans` looks harmless; `research` /
`research_codebase` / `codebase-research` / `Research` does not. Worth deciding
whether the wire and config vocabularies could collapse onto the linkage one.

## 5. YAML behaviour the port deliberately changed

Not bash inconsistencies, but oracle drift worth recording alongside the above,
since the old visualiser test tables were ported and rewritten rather than
carried over verbatim:

- A **trailing-whitespace quoted flow scalar** was reported `Malformed` by the
  visualiser only because libyml *crashed* on it. It is valid YAML;
  serde-saphyr parses it cleanly, and the crates now assert a clean parse.
- **Big integers** no longer widen. Values in `i64` range stay `Int`; beyond
  `i64` but within `u64` are preserved as `String`; beyond `u64` become `Float`.
  The old number-widening assertions became string-preservation assertions.
- **YAML-tagged values** are now `Malformed` rather than `Tagged`.

## Suggested reconciliation pass

Roughly in order of how much they can actually bite:

1. ~~Add the nested-manifest arm to `path_to_typed`~~ — **done**, see §3.
2. Decide whether `pr-description` is a linkage `target_type`; add the
   `linkage-type-pairs.tsv` row, or stop the awk typifying `meta/prs` paths.
   The two surfaces currently contradict each other, and which is wrong is a
   design decision rather than a bug fix.
3. Retire `lp_type_from_path` in favour of the config-driven
   `infer_type_from_path`, so bash linkage stops being blind to a re-pathed
   corpus. (The crates already are; this is about keeping the bash surface
   honest for as long as it survives.)
4. Consider collapsing the four doc-type vocabularies toward one.
