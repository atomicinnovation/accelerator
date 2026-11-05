# Prune simulator specification (PR 170)

The two specifications a reading pass wrote for PR 170, from which
`prune-simulator.js` was built: the root prune, then the tree prune. Paths
and lines refer to that PR's head, `6656b18a`.

## Part 1: root prune

Scope: the top level of a cache root, meaning version claims, single-file
prune, the residue sweep, the root lock and the gate in front of them.
Everything under `trees/` is specified in Part 2. Paths below are
relative to `cli/launcher/src/launch/outbound/resolve/` unless they start with
`cli/`, `bin/` or `docs-site/`. Line numbers are at head `6656b18a`.

### 1. Constants

| Name | Value | Source | Used for |
|---|---|---|---|
| `AGE_BACKSTOP` | 3,600 s (1 h) | `retention_windows.rs:7` | Minimum age before any entry, staging file, probe or lock directory can go |
| `CLAIM_WINDOW` | 1,209,600 s (14 d) | `retention_windows.rs:12` | Default claim window |
| `REFRESH_FRACTION` | 10 | `retention_windows.rs:18` | Refresh interval = `CLAIM_WINDOW / 10` |
| Refresh interval | 120,960 s (1.4 d) | `retention/version_claims.rs:48` | A valid claim younger than this is not rewritten |
| `--older-than <s>` | verb only, any `u64` | `retention/root_retention.rs:193` | Replaces `CLAIM_WINDOW` for version and tree claims; not clamped |
| `PID_LIMIT` | 64 bytes | `retention/residue.rs:15` | Longest `pid` file read; longer means keep |
| `LAUNCHER_ID_LENGTH` | 16 lowercase hex | `retention/version_claims.rs:20` | Claim-name suffix |
| `DIGEST_LENGTH` | 64 lowercase hex | `retention/root_entries.rs:23` | Sub-binary, shim and staging digests |
| `KNOWN_PLATFORMS` | `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64` | `mod.rs:61-62` | Launcher, shim and lock-directory names |
| `ROOT_LOCK` | `.accelerator-prune.lock` | `retention/root_lock.rs:15` | Root lock file |
| `VERSION_CLAIMS_DIR` | `claims` | `retention/version_claims.rs:18` | Claims directory |
| `TEMP_PREFIX` | `.tmp-` | `cli/kernel/src/lib.rs:15` | Staging names |

Time predicates (all ages saturate at 0, so a future mtime has age 0):

| Predicate | True when | Source |
|---|---|---|
| `aged(mtime)` | mtime known and `now - mtime >= 3600` | `retention_windows.rs:23-27` |
| `claimFresh(claim, window)` | claim valid and (mtime unknown or `now - mtime < window`) | `retention/version_claims.rs:159-176` |
| `claimStale(claim, window)` | claim valid and not `claimFresh` | `retention/single_file.rs:428-436` |
| `claimYoung(claim)` (start-up) | claim valid and mtime known and `now - mtime < 120960` | `retention/version_claims.rs:45-50` |

Edges: an entry exactly 3,600 s old is aged; a claim exactly at the window is
stale. An unknown mtime is never aged and always fresh. With `--older-than 0`
every claim with a known mtime is stale, including a future one.

### 2. Example names

`<dA>`, `<dB>`, `<dC>` are sub-binary digests; `<hA>`, `<hB>`, `<hC>` are
shim digests. Each is 64 lowercase hex, for example
`4e07408562bedb8b60ce05c1decfe3ad16b72230967de01f640b7e4729b49fce`. Launcher
ids are 16 lowercase hex, for example `0123456789abcdef`.

### 3. Simulator inputs

| Input | Values | Notes |
|---|---|---|
| Root kind | `default` (plugin `bin/`), `relocated`, `unidentified` | Chosen once at start-up (`retention/root_retention.rs:227-255`) |
| Root safe | yes / no | `SafePruneRoot::check`; relocated only |
| Trigger | `start-up`, `cold-store`, `cold-materialise`, `verb` (+ optional `--older-than`) | Cold paths use `Try`; the verb uses `Wait` |
| Running version | string, e.g. `1.26.0` | Launcher's `CARGO_PKG_VERSION` |
| Running shim per platform | a digest, or unhashable | SHA-256 of `<plugin_root>/bin/accelerator-verify-<platform>`; unhashable if no plugin root or unreadable |
| Root lock | free / held by another process / error (e.g. `ENOLCK`) / lock file is a symlink or foreign | |
| `claims/` state | absent / readable / unreadable (`readdir` fails) / untrusted (symlink, foreign owner, group-writable non-private, private group with ACL) / readable-but-unsearchable (per-claim `lstat` fails) | Last state models R1-06 |
| Each name | name, kind (file / dir / symlink), owner (self / other), mtime (known / unknown / future) | |
| Each lock directory | trusted or not, mtime, `pid` contents (missing / blank / live pid / dead pid / non-numeric / non-UTF-8 / > 64 bytes), extra files | |
| Live pids | set | `kill(pid, 0)`: `ESRCH` means dead; anything else alive |
| Clock | `now` | |

### 4. Gate

| Root kind | Root safe | Trigger | Claim written at start-up | Root prune runs | Verb result |
|---|---|---|---|---|---|
| default | n/a | start-up | no | n/a | n/a |
| default | n/a | cold-store / cold-materialise | no | no (materialise's own orphan reap only, see `Part 2`) | n/a |
| default | n/a | verb | no | tree prune `PinnedOnHost` only, `Wait` | `reclaimed N entries` |
| unidentified | n/a | any | no | no | error: cache root unavailable |
| relocated | no | start-up | yes | n/a | n/a |
| relocated | no | cold-store / cold-materialise | yes | no; warn once per process `not pruning the cache root` | n/a |
| relocated | no | verb | yes | no | `not pruning: <reason>`, non-zero, nothing on stdout |
| relocated | yes | cold-store / cold-materialise | yes | yes, `Try`, window `CLAIM_WINDOW` | n/a |
| relocated | yes | verb | yes | yes, `Wait`, window `--older-than` or `CLAIM_WINDOW` | see §11 |

Sources: `retention/root_retention.rs:44-79` (default), `:118-168` (cold
paths), `:170-196` (claim and verb), `:199-225` (unidentified),
`launch/cache.rs:155-170`.

### 5. Start-up claim refresh (relocated root only)

Runs once per process, every command, before any lookup
(`cli/launcher/src/launch/start_up.rs:72-78`).

1. Path = `claims/<running version>.<launcher id>`; launcher id = first 8 bytes
   of SHA-256 over `current_exe()`'s path, or over the version string if that
   fails (`retention/mod.rs:16-29`).
2. If the existing claim is valid (regular file, owned by self) and
   `claimYoung`, stop.
3. `mkdir claims` with mode 0700; `EEXIST` is fine, any other error stops.
4. If `claims/` is not a trusted directory, stop.
5. Open the claim `O_CREAT|O_TRUNC|O_NOFOLLOW` mode 0600, write nothing, set
   mtime to `now`. A symlink at the claim path fails here and stops.
6. Every stop after step 2 is logged at debug only; the command carries on.

Root safety is not consulted: an unsafe relocated root is still claimed.

### 6. Root lock

| Lock file state | `Try` (cold path) | `Wait` (verb) |
|---|---|---|
| Absent or own regular file, free | Take it | Take it |
| Own regular file, held elsewhere | Skip §7–§10 silently; tree prune still runs | Block until free, then take it |
| Symlink, directory, or not owned by self | Lock failure: skip §7–§10 | Lock failure: skip §7–§10 |
| `lstat` fails (not `NotFound`), or `flock` errors (e.g. `ENOLCK`) | Lock failure: skip §7–§10 | Lock failure: skip §7–§10 |

The lock is released at the end of §10, before tree prune takes any tree lock
(`retention/root_prune.rs:123-169`). A lock failure is reported as
`could not lock <root>/.accelerator-prune.lock: <detail>`.

### 7. Listing and classification

1. List the root once, under the lock. Drop names that are not UTF-8 (kept).
   A listing error is reported as `could not remove <root>: <kind>` and skips
   §8–§10.
2. Classify every name, first match wins (`retention/root_entries.rs:79-86`):

| Order | Kind | Shape | Example | Notes |
|---|---|---|---|---|
| 1 | `ShimStaging` | `accelerator-verify-<platform>-<64hex>.staging.<pid>` | `accelerator-verify-darwin-arm64-<hB>.staging.4242` | pid digits, ≥ 1 |
| 1 | `ShimCopy` | `accelerator-verify-<platform>-<64hex>` | `accelerator-verify-darwin-arm64-<hC>` | known platform |
| 2 | `LauncherEntry` | `accelerator-launcher-<version>-<platform>` with `<name>.minisig` listed | `accelerator-launcher-1.26.0-darwin-arm64` | version non-empty |
| 3 | `SubBinary` | `<prefix>-<64hex>`, prefix non-empty, not starting `.`, with `<name>.minisig` listed | `vcs-1.24.0-<dA>` | version never parsed out |
| 4 | `LauncherStaging` | `.tmp-launcher-<pid>` or `.tmp-launcher-<pid>.minisig` | `.tmp-launcher-4242` | pid ≥ 1 |
| 4 | `StoreStaging` | `.tmp-<stem>-<pid>-<seq>[.minisig]`, stem a digest-suffixed stem | `.tmp-vcs-1.24.0-<dA>-4242-0` | |
| 5 | `Probe` | `.accelerator-probe-<pid>` or `.accelerator-probe-<pid>-<seq>` | `.accelerator-probe-4242-3` | digits only |
| 5 | `BootstrapLock` | `.accelerator-lock-<platform>` | `.accelerator-lock-darwin-arm64` | known platform |
| 6 | `Unrecognised` | anything else | see below | always kept |

Always `Unrecognised` (from `retention/root_entries.rs:262-328` and the kept
names in the work item): `claims`, `trees`, `.accelerator-unverified.log`,
`.accelerator-prune.lock`, any `.minisig` file, an entry whose `.minisig` is
not listed, `accelerator-launcher-1.24.0-plan9-x64`, `.tmp-launcher-0`,
`.tmp-launcher-abc`, `.tmp-foreign`, `.tmp-vcs-1.24.0-<dA>-4242`,
`accelerator-verify-linux-x64-<63 hex>`, upper-case hex digests,
`.accelerator-probe-1-2-3`, `-<64hex>`, a bare `<64hex>`.

3. Running shims: for each platform that has at least one `ShimCopy` listed,
   hash the running plugin's source for that platform, or mark it unhashable.
4. Live bootstraps: for each pid in a `ShimStaging` or `LauncherStaging` name,
   alive unless `kill(pid, 0)` returns `ESRCH`.

### 8. Claims and the fresh set

Read after the listing (`retention/version_claims.rs:106-149`):

| `claims/` state | Listing | Effect |
|---|---|---|
| Absent | `Read([])` | No version is fresh except the running one |
| `lstat` of `claims/` fails (not `NotFound`) | `Unreadable(kind)` | Skip every versioned entry and every claim removal |
| Not a trusted directory | `Unreadable(permission denied)` | Same |
| `readdir` fails (not `NotFound`) | `Unreadable(kind)` | Same |
| Readable | `Read(claims)` | Per name below |

Per name in a readable `claims/`:

1. Parse `<version>.<16 lowercase hex>`, splitting at the last `.`; a name that
   does not parse is ignored (never protects, never removed).
2. `lstat` the claim. If that fails for any reason the claim is dropped
   silently and the listing stays `Read` (R1-06, line 138). Model the
   "readable but unsearchable" state as every claim dropped.
3. Valid = regular file and owner is self.

Fresh set = versions of valid claims that are `claimFresh(window)`, plus the
running version always.

### 9. Per-entry decisions

Process entries in listing order (unspecified in reality; sort for a
deterministic simulator). Age of an entry means the entry is a regular file
and `aged`; for kinds with a signature, the `.minisig` must also be a regular
file and `aged`.

| Kind | Skipped when claims unreadable | Protected when | Removed when | Removes, in order | Rule label |
|---|---|---|---|---|---|
| `SubBinary` | yes | stem minus `-<digest>` ends with `-<v>` for some fresh `v`, with at least one character before that `-` | unprotected and aged (entry and `.minisig`) | `.minisig`, then entry | `unclaimed aged sub-binary` |
| `LauncherEntry` | yes | its version is in the fresh set | unprotected and aged (entry and `.minisig`) | `.minisig`, then entry | `unclaimed aged launcher entry` |
| `ShimCopy` | no | platform unhashable, or digest equals the running shim's | unprotected and aged | the file | `aged shim copy of another release` |
| `ShimStaging` | no | its pid is alive | unprotected and aged | the file | `aged staging of a finished bootstrap` |
| `LauncherStaging` | no | its pid is alive | unprotected and aged | the file (each of the pair separately) | `aged staging of a finished bootstrap` |
| `StoreStaging` | no | never consulted | aged | the file (each of the pair separately) | `aged store staging` |
| `Probe` | no | n/a | never here (§10) | none | `kept` |
| `BootstrapLock` | no | n/a | never here (§10) | none | `kept` |
| `Unrecognised` | no | n/a | never | none | `kept` |

Failure handling for a pair: if the `.minisig` unlink fails, stop for this
entry, keep it, record the failure and count the attempt as not removed. If
the `.minisig` goes and the entry unlink fails, record the failure; the entry
is now unsigned and `Unrecognised` from the next prune on (R1-29). `NotFound`
on any unlink counts as removed (already gone).

Record, for every `SubBinary` and `LauncherEntry` processed, whether its
removal succeeded; §10 needs it. Entries kept for protection or youth count as
not removed.

Suffix-match examples (`retention/single_file.rs:117-123`, test at `:887`):
fresh `1.24.0-pre.78` protects `x-1.24.0-pre.78-<d>`; fresh `1.24.0` protects
`x-y-1.24.0-<d>` but `1.24.0-pre.78` does not.

### 10. Stale claims, then residue

Stale claims, only when the listing was `Read` (`retention/single_file.rs:401-459`):

For each valid claim that is `claimStale(window)`:

1. Keep it unless either another valid claim makes the same version fresh, or
   every `SubBinary` and `LauncherEntry` of that version processed in §9 was
   removed. A version with no listed entries satisfies the second condition.
2. Re-`lstat` the claim. Remove it only if it is still valid and stale. A
   refresh landing before this point keeps it.

Residue sweep, same listing (`retention/residue.rs:59-150`):

| Kind | Removed when | Removes, in order | Notes |
|---|---|---|---|
| `Probe` | regular file and aged | the file | no pid check |
| `BootstrapLock` | directory is trusted, its mtime is aged, and `pid` is missing, blank (whitespace only), or a positive integer whose process is dead; judged twice with identical results | `pid` (if present), then `rmdir` | non-UTF-8, non-numeric, ≤ 0, over 64 bytes, or a `pid` that is a symlink, directory or FIFO: keep |

`rmdir` fails with `directory not empty` when the lock directory holds
anything but `pid`; that is a removal failure on every prune.

Then release the root lock and run tree prune (`Part 2`). After a
materialise, tree prune first handles the materialised artifact under the lock
it already holds, releases it, and then runs §6–§10 with `Try` followed by tree
prune of every artifact (`retention/root_prune.rs:187-197`).

### 11. Report

Reclaimed count: each removed entry, claim, shim copy, staging file, store
staging file, probe and lock directory counts 1; a `.minisig` removed with its
entry and a lock directory's `pid` count 0. Tree removals add to the same count.

| Outcome | Verb stderr line | Affects exit | Cold path |
|---|---|---|---|
| Normal | none | exit 0, stdout `reclaimed N entries` | silent |
| Root lock held, `Try` | n/a | n/a | silent skip of §7–§10 |
| Root lock failure | `could not lock <root>/.accelerator-prune.lock: <detail>` | non-zero | `warn` with the same text |
| Claims unreadable | `claims unreadable (<kind>) in <root>/claims; kept everything they could protect` | no | `warn` |
| Untrusted tree directory | `not pruning through <path>: another user could write to it` | no | `warn` |
| Removal failure | `could not remove <path>: <kind>` | non-zero | `warn` |
| Root listing failure | `could not remove <root>: <kind>` | non-zero | `warn` |
| Unsafe root | `not pruning: <reason>` | non-zero, no stdout | `warn` once per process |
| Unidentified root | cache root unavailable error | non-zero | silent |

Stderr lines appear in the order locks, claims, untrusted, removals
(`retention/root_prune.rs:48-83`). A non-zero verb also prints
`the prune did not finish; see the failures above`
(`launch/cache.rs:174-190`). `<kind>` is Rust's `io::ErrorKind` text, such as
`permission denied` or `directory not empty`.

### 12. Races to annotate

These are timing windows, not states; show them as notes on the entries they
affect.

| Window | Actor | Outcome | Source |
|---|---|---|---|
| Between a dispatch's `find` and `exec`, its entry is removed | Any version | Claim-aware launcher re-resolves once; pre-claim launcher fails that dispatch | `retention/single_file.rs:382` |
| Between another version's shim hashing and `verify_launcher`, its shim copy is removed | Another version with different shim bytes | That bootstrap fails via `fail_integrity` and logs to `.accelerator-unverified.log`; next start re-stages | `bin/accelerator:300-330`, `:409-411` |
| Between the claims read and the entry unlink, a stale version starts and refreshes its claim | A version idle past the window | Its entries are still removed this time; the claim survives the re-check | `retention/single_file.rs:440-459` |
| Between the claim re-check and its unlink, a refresh lands | Any version | The refreshed claim is removed; rewritten at that install's next start | `retention/single_file.rs:416-424` |
| Between the second lock judgement and the `pid` unlink, a bootstrap reclaims and re-takes the lock | Bootstrap | Its lock is removed; two bootstraps may fetch concurrently, converging on the same bytes | `retention/residue.rs:86-112` |

### 13. Presets

Unless stated: relocated root, safe, running version `1.26.0` on
`darwin-arm64` with running shim `<hC>`, claims readable, root lock free,
`now` = T, no live pids except as listed.

#### P1. A/B/C upgrade (work item acceptance criterion)

Trigger: `cold-store` by C, which has just fetched and stored
`visualiser-1.26.0-<dC>`.

| Name | Age | Expected |
|---|---|---|
| `vcs-1.24.0-<dA>` + `.minisig` | 20 d | removed (unclaimed, aged) |
| `accelerator-launcher-1.24.0-darwin-arm64` + `.minisig` | 20 d | removed |
| `claims/1.24.0.0123456789abcdef` | 20 d | removed (every A entry removed) |
| `.tmp-vcs-1.24.0-<dA>-4242-0` | 20 d | removed (store staging) |
| `accelerator-verify-darwin-arm64-<hA>` | 20 d | removed (not the running shim) |
| `vcs-1.25.0-<dB>` + `.minisig` | 20 d | removed |
| `accelerator-launcher-1.25.0-darwin-arm64` + `.minisig` | 20 d | removed |
| `claims/1.25.0.fedcba9876543210` | 20 d | removed |
| `.tmp-vcs-1.25.0-<dB>-4343-0` | 20 d | removed |
| `accelerator-verify-darwin-arm64-<hB>` | 20 d | removed |
| `visualiser-1.26.0-<dC>` + `.minisig` | 0 | kept (running version, young) |
| `accelerator-launcher-1.26.0-darwin-arm64` + `.minisig` | 0 | kept |
| `claims/1.26.0.<C's id>` | 0 | kept (written at start-up) |
| `.tmp-visualiser-1.26.0-<dC>-5151-0` | 2 h | removed (store staging ignores protection) |
| `accelerator-verify-darwin-arm64-<hC>` | 0 | kept (running shim) |
| `.accelerator-probe-4242` | 2 h | removed |
| `.accelerator-lock-darwin-arm64/` with `pid` = dead pid | 2 h | removed |
| `.accelerator-unverified.log` | any | kept |
| `.accelerator-prune.lock` | created | kept |
| `claims/`, `trees/` | | kept; trees per `Part 2` |

Root-level reclaimed count: 13. Exit: dispatch proceeds to `exec`.

#### P2. Two live versions, different shims (R1-04)

Trigger: `cold-store` by `1.26.0`. Version `1.25.0` started 2 days ago.

| Name | Age | Expected |
|---|---|---|
| `claims/1.25.0.fedcba9876543210` | 2 d | kept (fresh) |
| `vcs-1.25.0-<dB>` + `.minisig` | 3 d | kept (protected by `1.25.0`) |
| `accelerator-launcher-1.25.0-darwin-arm64` + `.minisig` | 3 d | kept |
| `accelerator-verify-darwin-arm64-<hB>` | 3 d | removed (claims do not protect shim copies) |
| `accelerator-verify-darwin-arm64-<hC>` | 1 d | kept |

Annotate the shim-copy race from §12. If `<hB>` = `<hC>` the copy is one file
and kept.

#### P3. Pre-claim launcher alongside (R1-12)

Trigger: `cold-store` by `1.26.0`. `1.24.0-pre.88` has never written a claim.

| Name | Age | Expected |
|---|---|---|
| `vcs-1.24.0-pre.88-<dA>` + `.minisig` | 2 h | removed |
| `accelerator-launcher-1.24.0-pre.88-darwin-arm64` + `.minisig` | 2 h | removed |
| `browser-1.24.0-pre.88-<dB>` + `.minisig` | 30 min | kept (younger than `AGE_BACKSTOP`) |

Annotate: `1.24.0-pre.88` re-fetches on next use; a dispatch caught between
`find` and `exec` fails; offline its bootstrap fails through `fail_integrity`.
Its re-fetched entries are removed again by the next claim-aware prune once
1 h old.

#### P4. Unreadable claims directory

Trigger: `verb`. `claims/` is mode 0000 (or owned by another uid).

| Name | Age | Expected |
|---|---|---|
| `vcs-1.0.0-<dA>` + `.minisig` | 20 d | kept (versioned entries skipped) |
| `.tmp-vcs-1.0.0-<dA>-11-0` | 2 h | removed |
| `accelerator-verify-darwin-arm64-<hA>` | 2 h | removed |
| `.accelerator-probe-11` | 2 h | removed |

Stderr: `claims unreadable (permission denied) in <root>/claims; kept everything they could protect`. Exit 0.

#### P5. Readable but unsearchable claims directory (R1-06, as implemented)

Trigger: `verb`, not running as root. `claims/` is mode 0600, so it is trusted
and `readdir` works but every claim `lstat` fails with `EACCES`.

| Name | Age | Expected |
|---|---|---|
| `claims/1.25.0.fedcba9876543210` | 1 d | dropped from the listing; not removed |
| `vcs-1.25.0-<dB>` + `.minisig` | 3 d | removed, although `1.25.0` is claimed |
| `vcs-1.26.0-<dC>` + `.minisig` | 3 d | kept (running version) |

No warning, exit 0. The start-up claim write for `1.26.0` also fails (no
search permission), silently. If R1-06 were fixed, the listing would be
`Unreadable(permission denied)` and P5 would behave as P4.

#### P6. Root lock held (Try against Wait)

Another process holds `.accelerator-prune.lock`.

| Name | Age | `cold-store` | `verb` |
|---|---|---|---|
| `vcs-1.0.0-<dA>` + `.minisig` (unclaimed) | 20 d | kept (single-file skipped) | removed after the lock is released |
| `.accelerator-probe-11` | 2 h | kept (residue skipped) | removed after the lock is released |
| An unclaimed tree for an artifact whose tree lock is free | 20 d | removed (tree prune still runs) | removed |

The cold path never waits and reports nothing; the verb blocks until the other
holder releases.

#### P7. `--older-than` below the refresh interval (R1-27)

Trigger: `verb --older-than 86400`. Version `1.25.0` last started 20 h ago;
its claim was last written 30 h ago and was not rewritten because it was
younger than 1.4 d.

| Name | Age | `--older-than 86400` | No flag |
|---|---|---|---|
| `claims/1.25.0.fedcba9876543210` | 30 h | removed (stale, all entries gone) | kept |
| `vcs-1.25.0-<dB>` + `.minisig` | 3 d | removed | kept |
| `vcs-1.26.0-<dC>` + `.minisig` | 3 d | kept (running) | kept |

Documented at `docs-site/src/content/docs/internals.md:340-344`.

## Part 2: tree prune

Source of truth: PR 170 head `6656b18a`. Paths are relative to
`cli/launcher/src/launch/outbound/resolve/` unless stated. Everything below
was read from the code; nothing is inferred from docs alone.

### 1. Constants

| Name | Value | Source |
|---|---|---|
| `AGE_BACKSTOP` | 3,600 s (1 h) | `retention_windows.rs:7` |
| `CLAIM_WINDOW` | 1,209,600 s (14 d) | `retention_windows.rs:12` |
| `REFRESH_FRACTION` | 10, so a claim is rewritten once ≥ 120,960 s (~1.4 d) old | `retention_windows.rs:18`, `tree/claims.rs:202-208` |
| `LAYOUT_VERSION` | 1 | `tree/layout.rs:29` |
| `DIGEST_LEN` | 64 lowercase hex | `tree/layout.rs:48` |
| `SUFFIX_LEN` | 16 lowercase hex | `tree/layout.rs:47` |
| `LAUNCHER_ID_LENGTH` | 16 lowercase hex (`[0-9a-f]`) | `tree/claims.rs:28`, `195-200` |
| `POINTER_READ_LIMIT` | 256 bytes | `tree/layout.rs:366` |
| `KNOWN_PLATFORMS` | `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64` | `mod.rs:61-62` |
| `TEMP_PREFIX` | `.tmp-` | `cli/kernel/src/lib.rs:15` |
| Suffixes | `.ref`, `.sealed`, `.sealed.sig`, `.lease`, `.archive`, `.lock` | `tree/layout.rs:40-44`, `cli/lease/src/lib.rs:21` |
| Claim window used | verb: `--older-than` if given, else `CLAIM_WINDOW`; every cold path: `CLAIM_WINDOW` | `retention/root_retention.rs:72`, `141`, `163`, `193` |

Time rules (apply everywhere):

- `aged(mtime)` = mtime known **and** `now − mtime ≥ 3600`. Unknown mtime → not aged. Future mtime → age 0 → not aged. (`retention_windows.rs:23-27`)
- `claim_protects` = entry is a regular file **and** owner = effective uid **and** (mtime unknown **or** `now − mtime < window`). Future mtime → protects. (`tree/claims.rs:76-86`, `102-104`)
- `claim_stale` = regular file **and** owner = effective uid **and** mtime known **and** `now − mtime ≥ window`. Exactly at the window is stale. (`tree/claims.rs:90-100`)

### 2. On-disk layout

All under `<root>/trees/`. Digests abbreviated; real ones are 64 hex.

| Item | Example name | Grammar |
|---|---|---|
| Pointer | `browser-darwin-arm64-3f9a…c21e.ref` | `<name>-<platform>-<digest>.ref` |
| Generation | `browser-darwin-arm64-3f9a…c21e-1-0011223344556677/` | `<name>-<platform>-<digest>-<layout>-<suffix>` |
| Sidecar: attestation | `browser-darwin-arm64-3f9a…c21e-1-0011223344556677.sealed` | `<generation>.sealed` |
| Sidecar: signature | `…-1-0011223344556677.sealed.sig` | `<generation>.sealed.sig` |
| Sidecar: lease | `…-1-0011223344556677.lease` | `<generation>.lease` |
| Tree lock | `.tmp-browser-darwin-arm64.lock` | `.tmp-<name>-<platform>.lock` |
| Partial archive | `.tmp-browser-darwin-arm64-3f9a…c21e.archive` | `.tmp-<name>-<platform>-<digest>.archive` |
| Temp generation | `.tmp-browser-darwin-arm64-3f9a…c21e-1-89abcdef01234567/` | `.tmp-<generation>` |
| Temp pointer | `.tmp-browser-darwin-arm64-3f9a…c21e-1-89abcdef01234567.ref` | `.tmp-<generation>.ref` |
| Claims directory | `claims/` | literal |
| Tree claim | `claims/3f9a…c21e.9c4b1e0f2a7d5e83` | `<digest>.<launcher-id>` |
| Anything else | `backup-2021-1-final/`, `browser-plan9-x64-…` | kept, always |

Artifacts in examples: `browser`, `driver` (pinned), `legacy` (retired, no longer pinned).

### 3. Classification (`tree/layout.rs:315-361`)

Apply in order; first match wins.

1. Starts with `.tmp-`:
   - `.tmp-<name>-<platform>.lock`, platform known, name non-empty → **Lock**(artifact).
   - Else, after `.tmp-`, the first `-<known platform>` at offset > 0 that is followed by `-<64 hex>` and then `-` or `.archive` → **TempResidue**(artifact) (`Artifact::owning`, `layout.rs:79-88`).
   - Else **Unrecognised**.
2. Ends with `.ref`: split the stem at its last `-`; right part must be a well-formed digest; left part must be `<name>-<known platform>` → **Pointer**(artifact, digest). Else **Unrecognised**.
3. Ends with `.sealed.sig`, then `.sealed`, then `.lease` (that order): the prefix must parse as a generation name → **Sidecar**(artifact, generation). Else **Unrecognised**.
4. Parses as a generation name, any layout → **Generation**(artifact, digest, layout).
5. Otherwise **Unrecognised** (includes `claims`).

Generation name parsing (`layout.rs:266-290`): split from the right into `stem-digest-layout-suffix`; suffix 16 lowercase hex; digest 64 lowercase hex; layout all ASCII digits; stem must end in `-<known platform>` with a non-empty name before it. Non-UTF-8 names are never listed (kept).

### 4. Simulator state

| Object | Attributes |
|---|---|
| Root | kind `default` or `relocated`; relocated needs `safe` (passes `SafePruneRoot`) |
| `trees/` | `absent`, `ok`, `untrusted`, `unlistable` |
| `trees/claims/` | `absent`, `ok`, `unreadable` (list fails, e.g. mode `0000`), `no-search` (mode `0400`/`0600`: list works, every `lstat` fails), `untrusted` |
| Generation | mtime; lease liveness `Held` / `Free` / `Unknown`; held by this process (bool) |
| Pointer | contents string; file kind `regular` / `symlink` / `fifo` / `oversize` (> 256 B) / `non-utf8` |
| Sidecar, temp entry, lock | mtime; lock: held elsewhere (bool), lock call errors (bool) |
| Claim | digest, launcher id, owner `self`/`other`, kind `file`/`dir`/`symlink`, mtime, `lstat` fails (bool) |
| Running launcher | host platform; pinned digest per `(name, platform)`, on every platform (`resolver.rs:131-135`, `91-100`) |
| Trigger | `verb` / `cold-store` / `cold-materialise(X)`; window; this process's held leases |

Liveness of `<generation>.lease`: if the generation is in this process's held leases → `Held`, never probed (`tree/lease.rs:42-97`). Else: lease file missing → `Free`; filesystem whose locks are per process (e.g. NFS) or a probe error → `Unknown`; otherwise `Held` if any process holds it, else `Free` (`cli/lease/src/lib.rs:166-184`).

### 5. Survey (`tree/prune.rs:229-288`)

| Condition | Result |
|---|---|
| Relocated root fails `SafePruneRoot` | verb: refused, nothing removed; cold path: nothing, warning logged once per process |
| `trees/` absent | prune ends; nothing reported |
| `trees/` untrusted (not a directory owned by euid with no group/other write, unless group-writable through a private group with no ACL) | ends; report `not pruning through <root>/trees: another user could write to it` |
| `trees/` cannot be listed | ends; removal failure on `<root>/trees` |
| `claims/` absent | claims = empty `Read` |
| `claims/` `lstat` error, or list fails | claims = `Unreadable(kind)`; report `claims unreadable (<kind>) in <root>/trees/claims; kept everything they could protect` |
| `claims/` untrusted | claims = `Untrusted`; report `not pruning through <root>/trees/claims: another user could write to it` |
| `claims/` `no-search` | claims = `Read` with **every claim dropped** (each `lstat` fails; R1-08, `claims.rs:183`) |
| Claim name not `<64-hex>.<non-empty>`, or its `lstat` fails | not listed: protects nothing, never swept |

When claims are `Unreadable` or `Untrusted`: skip §7.2 (unclaimed trees) and §8.2 (stale sweep); everything else still runs.

### 6. Visit order and lock choice (`tree/prune.rs:121-180`, `290-365`)

Artifacts visited, sorted by `(name, platform)`:

- `default` root (`PinnedOnHost`): the running launcher's pinned names on the host platform.
- `relocated` root (`WholeTree`): every artifact named by any Pointer, Generation, Sidecar, TempResidue or Lock entry.

"Work" = the index holds a Pointer, Generation (any layout) or TempResidue of the artifact.

| Artifact state | `Wait` (verb) | `Try` (cold paths) |
|---|---|---|
| Has work | `take_tree`: create if missing, block → **Taken** | `try_take_tree`: create if missing, non-blocking; held elsewhere → **Skipped** |
| No work, lock file exists (relocated only) | `wait_existing_tree`: block, never create; vanished → **LockFree** | `probe_tree`: non-blocking, never create; held → **Skipped**; absent → **LockFree** |
| No work, sidecars only (relocated only) | no lock call → **LockFree** | same |
| Otherwise (or `default` root with no work) | **Skipped** | **Skipped** |
| Lock call errors (e.g. `ENOLCK`) | **Failed**: `could not lock <lock>: <detail>`; continue | same |

Per arm:

- **Taken**: §7.1 reap (only if no reap has run yet in this prune) → §7.2 if claims `Read` → relocated only: §7.3 orphan sidecars, then §7.4 orphan lock. Lock released at the end of the arm.
- **LockFree**: §7.3 orphan sidecars only.
- **Skipped**: nothing, no report.

A process never holds two tree locks while blocking: the reap's probes of other locks never block.

### 7. Per-artifact steps

#### 7.1 Orphan reap (`tree/reap.rs:137-226`)

Requires a reap permit: `trees/` trusted; relocated root also `safe`. Runs once per prune, under the first lock taken, and covers **all** artifacts.

Temp residue, grouped by owner artifact:

| Owner's lock | Temp generation / temp pointer | Partial archive |
|---|---|---|
| The lock this reap runs under | remove | remove, unless its digest is the owner's pin |
| Probed and taken (free) | remove | remove, unless pinned |
| No lock file | remove | **keep** |
| Held by another process | keep | keep |
| Probe errors | keep (warning) | keep |

No age gate on temp residue. Lock files are never removed here.

Unreferenced generations: `referenced` = the trimmed contents of every readable `*.ref` (bounded read, any contents, used only to keep). Remove a Generation iff layout = 1 **and** its name ∉ `referenced` **and** liveness ≠ `Held` **and** `aged(generation)`. Then unlink its three sidecars, ignoring errors.

Reap removals count +1 each on success (sidecars not counted). **Failures are silently ignored.**

#### 7.2 Unclaimed pointed trees (`tree/prune.rs:402-456`, `516-571`)

For each Pointer of the artifact, in name order:

1. Read contents: must be a regular file, not a symlink, FIFO or device, ≤ 256 bytes, UTF-8; else **Malformed** → keep.
2. Trim. Must contain no `/` or `\`, be ASCII, start with exactly `<name>-<platform>-<digest>-1-`, and end in 16 lowercase hex → **Current**(generation). Same identity but another layout number → **OtherLayout** → keep. Anything else (including `../victim`, `/abs/path`, another artifact's or digest's generation) → **Malformed** → keep.
3. Keep if digest = running launcher's pin for `(name, platform)`.
4. Keep if any listed claim on the digest protects (first claim read).
5. Liveness: `Held` → keep; `Free` → continue at any age; `Unknown` → continue only if `aged(generation)`.
6. Re-list claims. Not `Read`, or now protected → keep.
7. Unlink the pointer. Failure → report and stop for this pointer. Already gone → continue.
8. `try_exclusive_lease(<generation>.lease)` (creates the lease file if missing): a shared holder → stop (generation, sidecars and claims kept; the pointer is already gone). Error → continue **without** exclusion.
9. `remove_dir_all(generation)`: +1 reclaimed on success; failure reported, continue.
10. Unlink `.sealed`, `.sealed.sig`, `.lease`: not counted; failures reported.
11. Unlink every claim on the digest from the re-read listing, any launcher id: not counted; failures reported.

A Current pointer whose generation directory is missing is still removed (lease missing → `Free`; directory `NotFound` → not counted, not a failure).

#### 7.3 Orphan sidecars, relocated only (`tree/prune.rs:458-485`)

For each Sidecar of the artifact whose generation is not in the index: remove iff liveness ≠ `Held` **and** `aged(sidecar)`. +1 per sidecar removed; failures reported.

#### 7.4 Orphan tree lock, relocated only (`tree/prune.rs:487-514`)

Only in the **Taken** arm. Remove the artifact's lock file iff the index (less this prune's removals) holds no work for it **and** `aged(lock)` **and** a fresh listing of `trees/` confirms no work. +1; failure reported. The lock is released after the unlink.

### 8. After the loop

1. Fallback reap (`tree/prune.rs:166-202`): if no reap ran and the index holds TempResidue or a Generation no pointer names, take the first pinned artifact's lock (creating it, with the trigger's patience) and run §7.1.
2. Stale-claim sweep, relocated only, claims `Read` (`tree/prune.rs:575-604`): for each listed claim with a 16-hex id and `claim_stale`, `lstat` it again; still stale → unlink, +1. Missing → skip silently. No lock held.

### 9. Triggers

| Trigger | Root | Sequence |
|---|---|---|
| `cache prune` | default | Tree prune `PinnedOnHost`, `Wait`, window = `--older-than` or 14 d. No root prune. |
| `cache prune` | relocated | `safe` check → single-file root prune under root lock (`Wait`) → root lock released → tree prune `WholeTree`, `Wait`. |
| Cold store | default | Nothing. |
| Cold store | relocated | Single-file root prune (`Try`) → tree prune `WholeTree`, `Try`, held leases = this dispatch's acquired trees. |
| Cold materialise of X | default | Materialise under X's lock → §7.1 reap with a `default` permit → release. No §7.2. |
| Cold materialise of X | relocated | Materialise under X's lock → §7.1 reap (permit needs `safe`) → `prune_held`: §7.2 and §7.3 for X only → release X's lock → full prune: single files (`Try`) then `WholeTree` (`Try`), X included. |
| `cache repair X` | either | Superseded generation: `Free` → remove it and its sidecars now; `Unknown` → touch its mtime (restart backstop); `Held` → keep. Needs a permit. |

`Try` never waits: a held root or tree lock is skipped with no report. `Wait` blocks on each lock in turn.

### 10. Report

| Field | Message | Exit effect (verb) |
|---|---|---|
| reclaimed | `reclaimed N entries` | — |
| lock failure | `could not lock <path>: <detail>` | non-zero |
| removal failure | `could not remove <path>: <kind>` | non-zero |
| unreadable claims | `claims unreadable (<kind>) in <dir>; kept everything they could protect` | zero |
| untrusted directory | `not pruning through <dir>: another user could write to it` | zero |

Counted once: a removed generation (pointed or reaped), a reaped temp entry, an orphan sidecar, an orphan lock, a stale claim. Not counted: pointers, sidecars and claims removed with their tree. `NotFound` on any removal is neither counted nor a failure. Cold paths log each problem as a warning and never fail the dispatch. A reap error becomes `could not remove <root>/trees: other error` (`prune.rs:394-399`).

### 11. Presets

Host `darwin-arm64`. Running launcher pins `browser@C` and `driver@E` on every platform. "Aged" = 3 h old; "young" = 10 min old. Leases `Free` unless stated.

#### P1 — Two unclaimed browser trees (relocated, cold store)

| Entry | State |
|---|---|
| `browser@A` pointer, generation (aged), 3 sidecars | claim `A.9c4b1e0f2a7d5e83` 20 d old |
| `browser@B` pointer, generation (aged), 3 sidecars | no claim |
| `browser@C` pointer, generation, 3 sidecars | claim `C.1a2b3c4d5e6f7081` 1 h old |
| `.tmp-browser-darwin-arm64.lock` | 30 d old |

Expected: A and B pointers, generations, sidecars and claim `A.…` removed; C and its claim kept; lock kept (browser still has work). Stale sweep finds `A.…` already gone and skips it. Report: `reclaimed 2 entries`, no problems. Same result from the verb.

#### P2 — Pointers that are never followed (relocated, verb)

| Pointer | Contents | Classified |
|---|---|---|
| `browser-darwin-arm64-D1.ref` | `../victim` (`<root>/victim/` exists) | Malformed |
| `browser-darwin-arm64-D2.ref` | `/Users/me/victim` | Malformed |
| `browser-darwin-arm64-D3.ref` | `driver-darwin-arm64-D3-1-0011223344556677` (exists) | Malformed |
| `browser-darwin-arm64-D4.ref` | symlink to a file with valid contents | Malformed |
| `browser-darwin-arm64-D5.ref` | `browser-darwin-arm64-D5-2-0011223344556677` (exists) | OtherLayout |
| `browser-darwin-arm64-D6.ref` | `browser-darwin-arm64-D6-1-0011223344556677`, aged, unclaimed | Current |

Expected: D1–D5 pointers kept; both victims untouched; the `driver` and layout-2 directories kept (the reap sees them as referenced, and layout 2 is never reaped); D6 tree removed. Report: `reclaimed 1 entries`.

#### P3 — A dispatch leases during removal (relocated, verb)

`browser@A` aged, unclaimed, `Free` at the probe. Event: a dispatch takes a shared lease on A's generation after step 6 and before step 7.

Expected: pointer removed; exclusive lease refused; generation, sidecars and claims kept; `reclaimed 0 entries`, no problems. Next prune once the dispatch has exited and the generation is aged: A's generation is unreferenced → reaped with its sidecars. Its installation re-fetches on the next dispatch.

Variant P3b: the dispatch asks for the lease after step 8. It blocks until step 11, then fails its identity check, retries, misses, and re-materialises. The prune removes everything.

#### P4 — Claims directory cannot be read (relocated, verb)

`browser@S` aged, lease `Free`, claim `S.…` 1 h old (a sibling installation's); `browser@C` (pinned); `.tmp-legacy-darwin-arm64-F-1-…/` aged temp residue, no `legacy` lock file.

- P4a, `claims/` mode `0000`: listing `Unreadable`. `browser@S` and `browser@C` kept; temp residue reaped (owner lock absent); no `legacy` lock is created, because the reap's removal is forgotten by the index before `legacy` is visited. Report `claims unreadable (permission denied) in <root>/trees/claims; kept everything they could protect`; exit 0.
- P4b, `claims/` mode `0600`: the directory is trusted and listable, but every claim's `lstat` fails, so both listings are `Read` and empty. `browser@S` (pointer, generation, sidecars) is removed; its claim `S.…` is not in the re-read listing, so no claim unlink is attempted and nothing is reported; exit 0. `browser@C` kept as the pin. This is R1-08: the removal is silent.

#### P5 — Scope: default root against relocated root (verb)

Same entries in both roots:

| Entry | State |
|---|---|
| `browser@old` (host) | aged, unclaimed |
| `legacy` (host, retired) tree + `.tmp-legacy-darwin-arm64.lock` | aged, unclaimed; lock 2 h old |
| `browser` on `linux-x64` | aged, unclaimed, no lock file |
| 3 sidecars of `browser-darwin-arm64-G-1-…`, no generation | aged |
| claim `Z.0123456789abcdef`, no tree on `Z` | 20 d old |
| `backup-2021-1-final/` | aged |

| Entry | Default root | Relocated root |
|---|---|---|
| `browser@old` | removed | removed |
| `legacy` tree | kept | removed |
| `legacy` lock | kept | removed in the same run |
| `browser` on `linux-x64` | kept | removed; its lock is created (young) and goes on a prune ≥ 1 h later |
| Orphan sidecars | kept | removed (3) |
| Claim `Z.…` | kept | removed |
| `backup-2021-1-final/` | kept | kept |
| Report | `reclaimed 1 entries` | `reclaimed 8 entries` |

#### P6 — A held tree lock: cold path against verb (relocated)

Another process holds `.tmp-browser-darwin-arm64.lock`. `browser@A` and `driver@B` aged, unclaimed.

- Cold store (`Try`): `browser` skipped with no report; `driver@B` removed, and the reap runs under driver's lock, keeping any browser temp residue (its lock is held elsewhere). `reclaimed 1 entries`.
- Verb (`Wait`): blocks until the holder releases, then removes `browser@A` and `driver@B`. `reclaimed 2 entries`.
- Verb with the browser lock call erroring (`ENOLCK`): `could not lock <root>/trees/.tmp-browser-darwin-arm64.lock: …`; `driver@B` still removed; exit non-zero.
