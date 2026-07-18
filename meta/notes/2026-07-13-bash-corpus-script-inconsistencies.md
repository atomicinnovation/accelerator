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

The practical consequence: **linkage was not config-aware.** A repository that
re-paths its corpus (`meta/work` → something else) got correct validation and
correct migration, but linkage silently stopped resolving — references degraded to
raw paths in the `ambiguous` band, and nothing detected it.

**Fixed on both sides.** `corpus::linkage` takes the same injected doc-type table
the other surfaces take and derives path→type through `corpus::doc_type::infer`
(longest-match), single-sourced on `DocTypeKey`. And `linkage-parser.sh` now
*sources* `doc-type-inference.sh` rather than carrying its own table:
`lp_type_from_path` is gone. The hand-ordered review-before-generic arms went with
it — longest-configured-dir-wins makes that ordering fall out instead of needing to
be maintained.

Two things fell out of the bash retirement, both worth knowing:

- The parser is invoked **once per corpus file** by the 0007 migration, so
  resolving config on startup spawned the resolver per file. The migration's
  resolve-once guard caught it. `load_doc_type_table` now accepts a pre-resolved
  table via `DOC_TYPE_TABLE_TSV`, which the migration hands down.
- Type inference is config-aware now, but **token extraction still is not** — see
  §5.

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

**Resolved** — `pr-description` is now a legal linkage `target_type`:
`linkage-type-pairs.tsv` gained `work-item relates_to pr-description` and `plan
relates_to pr-description`. Landing that row *first* would have been a mistake,
though — see §2a, which had to be fixed before the row was safe.

## 2a. The pr-description id: the PR number, or the filename stem? **[FIXED]**

Adding the pairs row promotes a `meta/prs` reference from `ambiguous` to
`resolved` — which asserts the reference points at a real document. It did not.

A pr-description is identified by its **PR number**. `templates/pr-description.md`
says so outright (`id: "{pr_number}"` — "PR number as a quoted YAML string"),
`describe-pr` writes it that way, and every pr-description in the corpus carries a
bare-number id (`12`, `14`, `16`, `18`).

Two surfaces disagreed:

- `path_to_typed` derived a reference's id from the **filename stem** —
  `meta/prs/12-description.md` → `pr-description:12-description`, an id no
  document has.
- the migration's `derive_stem` did the same, so a *backfilled* pr-description
  would have been written `id: "12-description"`, in defiance of its own template.

So a resolved reference would have pointed at nothing, and a backfilled document
would have contradicted the template it was backfilled from. Resolved-and-dangling
is worse than ambiguous: ambiguous at least reports itself as unverified.

**Fixed** — the *stem* and the *identity* are now separate concepts. `derive_stem`
stays the human-readable stem that title/date/`extra_default` read; a new
`derive_id` supplies the identity, and for a pr-description it takes the number
from `extra_default`'s `pr_number` rule rather than a second copy of it — so the
id and the `pr_number` field cannot disagree. `path_to_typed` mirrors that rule.
Both now handle `pr-42-description.md` → `42` (a genuine `pr-` segment) as well as
`12-description.md` → `12`, and neither mistakes a date-prefixed stem's year for a
PR number.

Conflating the two is what made the first attempt fail: `extra_default pr_number`
derives *from the stem*, so shortening the stem to `12` starved it and stamped
`pr_number: unknown`.

Because a reference is resolved from the path alone, the document's id must equal
what the path predicts. The migration therefore now **coerces** a pre-existing
stem-shaped id (`416-summary` → `416`) via a new `canonical_id` channel to the
rewrite awk, with a `0007-DIVERGE[id-canonicalised]` breadcrumb so the rewrite is
auditable rather than silent.

`pr-review` is untouched throughout: `meta/reviews/prs` is the longer configured
directory, so most-specific match still wins and PR reviews keep their full-stem
ids.

## 3a. `normalize_paths` corrupted every reference whose type ran `match()` **[FIXED]**

Found while fixing §2a, and **entirely pre-existing** — nothing above caused it.

`normalize_paths` rewrites a path token in place, using awk's `RSTART`/`RLENGTH` to
splice around it. But `RSTART`/`RLENGTH` are **globals**, and `path_to_typed` runs
`match()` itself. So the splice read offsets left behind by the *inner* match:

```
["meta/decisions/ADR-0026-old.md"]  →  ["adr:ADR-0026"ecisions/ADR-0026-old.md"]
```

Any ADR path reference inside a linkage value was mangled into malformed YAML.
`work-item` and `plan` escaped only by accident — their arms use `sub()`, which
does not set `RSTART`. No fixture had an ADR path reference in a linkage value, so
196 assertions passed straight over a live data-corruption bug.

**Fixed** — the token's offsets are saved before the call. A probe now pins every
arm, including a multi-token value and the unmapped-path passthrough.

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
2. ~~Decide whether `pr-description` is a linkage `target_type`~~ — **done**, see
   §2 and §2a. It is, via `work-item relates_to pr-description` and `plan
   relates_to pr-description`; the id derivation was fixed first so the newly
   resolvable references point at documents that exist.
3. ~~Retire `lp_type_from_path` in favour of the config-driven
   `infer_type_from_path`~~ — **done**, see §1. `linkage-parser.sh` now sources
   the shared matcher; `meta/prs` resolves, and a re-pathed corpus is classified
   correctly. Two things fell out of it, both recorded below.
4. ~~Token extraction is still hardcoded to `meta/`~~ — **done**, see §5.
5. The four doc-type vocabularies are **not** worth collapsing — see §6.

## 5. Path-token extraction was `meta/`-bound **[FIXED]**

Retiring `lp_type_from_path` made *type inference* config-driven, but the step
before it — deciding which strings in a line are candidate references at all — was
still a literal `meta/` prefix, in **both** implementations:

- `scripts/linkage-parser.sh`: `grep -oE 'meta/[A-Za-z0-9/_.-]+\.md'`
- `cli/corpus/src/linkage.rs`: `line[cursor..].find("meta/")`

So a corpus configured to `paths.work: docs/tickets` had its *source type*
classified correctly, but a reference *to* `docs/tickets/0002-y.md` was never
extracted — the scan never saw it. The document yielded no linkage records at all.

**Fixed on both sides.** The scan roots are now the distinct *leading segments* of
the configured doc-type directories, derived from the injected table
(`corpus::linkage::path_roots`, and `lp_build_path_re` in bash). Scanning by root
rather than by full directory deliberately preserves the old behaviour for an
out-of-scope subtree: `meta/docs/…` is still extracted, still fails to infer a
type, and is still carried through as a raw path ref.

The classifier's `meta/*.md` guard went too — the extractor only ever yields path
tokens ending `.md` (ADR ids, `pr:` refs and bare ids never do), so the suffix
identifies a path without pinning it to a root.

**The resolve-once invariant is load-bearing.** The 0007 migration invokes the
linkage parser once per corpus file, so having the parser resolve config on
startup spawned the resolver per file. A migration guard caught it. The shared
loader now takes a pre-resolved table via `DOC_TYPE_TABLE_TSV`, which the
migration hands down. Any future caller that shells to the parser in a loop needs
the same seam.

## 6. The four vocabularies should stay four

Recorded as a decision, against the earlier suggestion to collapse them.

They are not four spellings of an internal name — they are **three external
contracts** plus a display string, and each has a different consumer:

| Vocabulary | Example | Consumer | Cost of changing |
| --- | --- | --- | --- |
| `config_path_key` | `paths.research_codebase` | `config-defaults.sh`, every user's `.accelerator/config.md` | breaks every user config |
| `linkage_type_name` | `work-item:0046` | every typed ref in every document | breaks every corpus |
| `wire_str` | `work-items` | the visualiser HTTP API and the React frontend | breaks the API + frontend |
| `label` | `Work items` | humans | — |

Collapsing them means three simultaneous breaking migrations — config, corpus, and
wire — and the result would be *worse*, because the three forms are idiomatic for
their contexts on purpose: a config key naming a directory (`work`), a singular
typed reference (`work-item:0046`), and a plural collection on the wire
(`work-items`). Forcing one string would make at least two of them read wrong.

The problem worth solving was never the plurality — it was **drift**. That is now
closed: all four are `const fn`s on `DocTypeKey`, and the bash surfaces are pinned
against it by the `doc_type_single_source` suite, which now also asserts that every
`config_path_key` exists in `config-defaults.sh`'s `PATH_KEYS` (a key renamed in
bash would otherwise silently drop that type from the table, and the document would
simply stop being classified).

The one contract still unpinned is `wire_str` versus the frontend's TypeScript —
a cross-language pin, and the visualiser's own tests cover the wire today.

## A pattern worth naming

Most of these findings are one mistake wearing different clothes: **a fact about a
doc type, written down once per surface that needs it, and then allowed to drift.**
The dir→type map (§1), the design-inventory id (§3), and the pr-description id
(§2a) each had two or three encodings that disagreed — and in every case the
*reference* side disagreed with the *definition* side, so documents got correct ids
and then links to them pointed somewhere else.

The lesson from §2a is sharper than "don't duplicate": **an id that references are
resolved from must be derivable from the path.** `path_to_typed` sees only a path,
so any identity convention it cannot predict guarantees dangling references. That
is *why* the id has to be single-sourced, not merely tidier.

§3a is a different animal — a plain latent bug (awk globals clobbered across a call)
that only surfaced because the corrected id changed a string's length. It had been
corrupting ADR path references for as long as that arm has existed.

What every one of them has in common is that **the test suite was green**. Each bug
lived in an arm no fixture exercised. The crates now single-source these facts on
`DocTypeKey`, and four suites keep the bash surfaces honest against it: the doc-type
registry, the rewrite awk's `path_to_typed`, `normalize_paths`' splicing, and
`linkage-type-pairs.tsv` versus `corpus::linkage::TYPE_PAIRS`. Every fact still
written twice wants a test like those, or it will drift again — silently, and with
the suite still green.
