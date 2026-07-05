---
type: codebase-research
id: "2026-07-06-0165-multi-binary-distribution-release-pipeline"
title: "Research: Producer-side multi-binary distribution and release pipeline with minisign (0165)"
date: "2026-07-05T23:35:11+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0165"
parent: "work-item:0165"
relates_to: ["codebase-research:2026-07-03-0164-launcher-and-git-style-dispatch", "codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
topic: "How to build the producer half of the static-binary release pipeline that satisfies the launcher's frozen manifest.json + minisign contract"
tags: [research, codebase, distribution, release, minisign, cross-compile, rust, version-coherence]
revision: 4608507878edcffecab5d28e3945b4a0b90d0dd0
repository: accelerator
last_updated: "2026-07-05T23:35:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Producer-side multi-binary distribution and release pipeline with minisign (0165)

**Date**: 2026-07-06 (UTC 2026-07-05T23:35:11+00:00)
**Author**: Toby Clemson
**Git Commit**: 4608507878edcffecab5d28e3945b4a0b90d0dd0
**Repository**: accelerator

## Research Question

For work item [0165](../../work/0165-multi-binary-distribution-and-release-pipeline.md):
what does the codebase look like today, and what must change, to build the
**producer half** of on-demand static-binary distribution — a release pipeline
that cross-compiles every workspace binary, emits a signed `manifest.json` that
satisfies the launcher's already-frozen consumer contract (0164), signs
everything with minisign, retires the flat `checksums.json`, and enforces
version coherence?

## Summary

The consumer side is **fully implemented and frozen** in `cli/launcher` (0164):
the launcher fetches `manifest.json` + `manifest.minisig`, enforces
`schema_version <= 1`, exact-equality anti-rollback on `version`, per-binary
`{sha256, signature}`, and minisign verification against a build-embedded public
key. Every field name and string value is load-bearing. The **producer side is
stale**: `tasks/build.py` + `tasks/github.py` cross-compile a *single*
hardcoded binary (`accelerator-visualiser`), emit a flat
`checksums.json` (`platform → "sha256:hex"`, no descriptions, no signatures, no
`schema_version`), and contain **zero minisign code**. The work is to lift the
producer up to the frozen contract.

Two findings materially reshape how 0165 should be planned — **read the
"Critical reconciliations" section before writing the plan**:

1. **There are no `accelerator-<sub>` sub-binaries in the workspace yet, and
   none carry a `package.description`.** The cli workspace builds exactly two
   production binaries (`accelerator`, `accelerator-verify`) plus a `kernel`
   library. The visualiser — the intended "first concrete sub-binary" — is
   *not* a workspace member; 0168 folds it in. So the manifest's `binaries` map
   is legitimately **empty** at current HEAD, and the "description sourced from
   each crate's `Cargo.toml` `package.description`" requirement has no data
   source until crates gain descriptions.
2. **The work item's "sign whole-file (not `-H` prehashed)" instruction needs
   careful reading against the launcher's `allow_legacy = false`** — they *do*
   reconcile (modern minisign `-S` prehashes by default), and the existing
   `cli/verify` tests prove a plain `minisign -S` round-trips, but the phrasing
   is a trap worth pinning down empirically before relying on it.

## Detailed Findings

### 1. The frozen consumer contract (what the producer MUST emit)

Source of truth: `cli/launcher/src/launch/outbound/resolve/` + the golden
fixtures. **Do not modify these — the pipeline conforms to them.**

**Manifest schema** (`manifest.rs:21-46`, formalised in
`cli/launcher/tests/fixtures/manifest.schema.json`):

```
{
  "schema_version": 1,                     // u64, required, minimum 1
  "version": "1.24.0-pre.7",               // String, required
  "binaries": {                            // BTreeMap, #[serde(default)] (empty is legal)
    "<binary-name>": {
      "description": "…",                  // String, #[serde(default)] (optional)
      "platforms": {                       // BTreeMap, #[serde(default)]
        "darwin-arm64": {
          "sha256": "…64-hex…",            // String, REQUIRED, no default
          "signature": "…minisig…"         // String, REQUIRED, no default
        }
      }
    }
  }
}
```

- Top-level `required`: `["schema_version", "version", "binaries"]`
  (`manifest.schema.json:7`). `platformEntry.required`: `["sha256", "signature"]`
  (`:45`). `binaryEntry.required`: `["platforms"]` — `description` is optional
  in the schema but AC2 requires it non-empty.
- `sha256` pattern `^(sha256:)?[0-9a-f]{64}$` (`manifest.schema.json:50`) —
  lowercase hex, optional `sha256:` prefix (`bare_sha256()`, `manifest.rs:48-66`).
- Platform keys are constrained to exactly
  `["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64"]`
  (`manifest.schema.json:36-38`; host aliases at `mod.rs:20-28`).

**Validation gates** (`manifest.rs:77-110`, `parse_and_validate`):
1. `schema_version > SUPPORTED_SCHEMA_VERSION (=1)` → `UnsupportedSchema`
   (strictly-higher only; lower passes).
2. `manifest.version != expected_version` → `ManifestVersionMismatch`, where
   `expected_version = env!("CARGO_PKG_VERSION")` of the launcher crate
   (`mod.rs:42`). **Literal string equality — this is the anti-rollback.**
3. All-zeros `SENTINEL_SHA256` (64 zeros) marks a platform as intentionally
   absent → `AssetNotFound` (`manifest.rs:15-17, 48-66`).

**Verification order** (`verifier.rs`):
- `verify_binary` (`:29-50`): compute lowercase-hex sha256 → compare to
  `expected_sha256` (corruption check) → `keys.verifies(bytes, signature)` (the
  security boundary). sha256 first, signature second.
- `verify_manifest` (`:57-67`): signature-only over the raw manifest bytes, **no
  sha256**. Called *before parsing* (`mod.rs:129-130`): "Verify the signature
  over the raw bytes before parsing anything."
- The minisign call is `key.verify(data, &parsed, false)` (`keys.rs:68`); the
  `false` is `allow_legacy`. `Signature::decode(signature)` parses the sig
  (`keys.rs:63`). **See Critical Reconciliation #2 for what `allow_legacy=false`
  demands of the signer.**

**Embedded key** (`build.rs:28-45` → `keys.rs:11-12`): `build.rs` reads
`../../keys/accelerator-release.pub` (repo root) into `$OUT_DIR/release.pub`,
`include_str!`'d as `EMBEDDED_RELEASE_KEY`, with `rerun-if-changed` on the
source. `TrustedKeys` does **verify-any-of** across a `Vec<PublicKey>`
(`keys.rs:62-69`) but production uses just the one embedded key
(`embedded()`, `:55-57`). Committed key (`keys/accelerator-release.pub`):

```
untrusted comment: minisign public key 001BAF670F78DDFD
RWT93XgPZ68bAIL3VqyfTRldzOQgGIjg6k6cipn3Ppgh1xB3lgJ93ae0
```

Key ID `001BAF670F78DDFD`. This is a **structurally-valid** minisign public key
(parses, has the `RW` Ed25519 prefix, guarded by tests
`keys.rs:76-85`), *not* a bare placeholder string — but the file cannot tell us
whether its secret half is a real production signer or a throwaway. The work
item treats it as a placeholder to be replaced; whether the committed value
literally changes depends on whether the secret half already exists (see
Critical Reconciliation #3).

**Signature string format**: the `signature` field is the **entire four-line
`.minisig` file content**, newline-joined:

```
untrusted comment: signature from accelerator release key\nRWT…\ntrusted comment: …\nAAAA…\n
```

(`manifest.example.json:6-…`, `manifest.schema.json:54`).

**Asset naming & release layout** (`mod.rs:118-144`, `main.rs:34-43`): assets
are flat under the GitHub release tag `v{version}`
(`https://github.com/atomicinnovation/accelerator/releases/download/v{version}`,
overridable via `ACCELERATOR_RELEASE_BASE_URL`):
- `manifest.json`
- `manifest.minisig`
- one `{name}-{platform}` binary per manifest entry — **no file extension**
  (`format!("{name}-{}", platform)`, `mod.rs:143-144`), e.g.
  `accelerator-visualiser-darwin-arm64`.

**The verify shim** (`cli/verify/src/main.rs`, crate `accelerator-verify`): CLI
`accelerator-verify <pubkey> <sig> <target>`, `public_key.verify(&target,
&sig, false)` (`main.rs:41`), exit 0 iff valid. Same `allow_legacy=false`.
Its tests (`cli/verify/tests/verify.rs:33-56`) shell out to the real `minisign`:
`minisign -G -W -f -p <pub> -s <secret>` to generate, `minisign -S -s <secret>
-x <sig> -m <target>` to sign — **no `-H`** — and assert the shim verifies.
This is the empirical proof that a plain `-W` key + plain `-S` round-trips.

### 2. The stale producer pipeline (what exists today)

Python invoke tasks under `tasks/`, orchestrated by `mise run`.

**Cross-compile loop** (`build.py:201-215`, `server_cross_compile`):
- Iterates `for triple, platform in TARGETS` (four targets, `targets.py:1-6`).
- `cargo zigbuild --release --target {triple} --manifest-path {CARGO_TOML}`
  where `CARGO_TOML = skills/visualisation/visualise/server/Cargo.toml`
  (`paths.py:9-10`).
- **Single-binary hardcode**: `src = SERVER/target/{triple}/release/accelerator-visualiser`
  (`build.py:213`, bare literal) → magic-byte check → staged to
  `binary_path(platform)` = `bin/accelerator-visualiser-{platform}`
  (`paths.py:39-40`).
- Debug archives mirror it (`build.py:218-225`; `debug_archive_path`,
  `paths.py:43-44`).

**Magic-byte check** (`build.py:104-115`): first 4 bytes; darwin triples must be
in `_MACHO_MAGIC` (`cf/ce faedfe`, `cafebabe`); everything else must be
`\x7fELF`. Platform inferred from the substring `"darwin"` in the triple. **No
static-linking assertion yet** — 0165 must add the `readelf` check (no
`PT_INTERP`, no `DT_NEEDED`; do NOT assert ELF type `EXEC` — musl static-PIE is
`ET_DYN`).

**checksums.json** (`build.py:118-128` writer, `:228-239` `create_checksums`):
format is `{ "version": "<semver>", "binaries": { "<platform>": "sha256:<hex>" } }`.
`compute_sha256` streams 64 KiB chunks, lowercase hex (`hashing.py:5-10`). This
is the file 0165 **retires entirely** in favour of `manifest.json`.

**Version coherence** (`build.py:131-151`, `validate_version_coherence`) compares
`expected_version` against **five** sources — already broader than the work item
implies:
1. `.claude-plugin/plugin.json` `version` (`build.py:49-51`)
2. server `Cargo.toml` `[package].version` (`:54-55`)
3. `bin/checksums.json` `version` (`:58-60`) — **this reader goes away with
   checksums.json retirement; replace with `manifest.json` version**
4. `cli/Cargo.toml` `[workspace.package].version` (`:63-71`)
5. any `cli/` workspace member that **pins** its own literal `[package].version`
   (members inheriting via `version.workspace = true` contribute nothing)
   (`:74-101`)

Any mismatch → `VersionCoherenceError`. This is the seam to extend so
`manifest.version` joins the coherence set (the launcher enforces
`manifest.version == launcher CARGO_PKG_VERSION`).

**Upload + re-verify + draft-preserve** (`github.py:136-178`,
`upload_and_verify`):
- `tag = f"v{version}"`; reads `checksums.json`, strips `sha256:` into per-platform
  hashes.
- Upload loop (`:159-161`): `gh release upload {tag} {path}` per binary + debug
  archive.
- Re-verify loop (`:162-163`): `download_and_verify` re-downloads each **binary**
  (not archives) to a temp file, recomputes sha256, compares. Mismatch →
  `AssetVerificationError`.
- Publish on success (`:164`): `gh release edit {tag} --draft=false`.
- **Draft-preserve seam** (`:165-171`): on `AssetVerificationError`, emit a
  forensic GitHub-Actions annotation and **re-raise without deleting** — release
  stays draft + tag preserved for triage. Any *other* exception (`:172-177`)
  deletes the release + tag. This is exactly the behaviour AC "release remains
  in draft on verification failure" needs; 0165 extends it to also upload
  `manifest.json` + every `.minisig` and to re-verify against manifest hashes.

**No producer minisign code exists** — confirmed by grep across `tasks/` and
`.github/workflows/`. The only minisign references outside `cli/` are the mise
tool provisioning (`mise.toml:29,32`, pins `jedisct1/minisign` `0.12` via ubi),
the `.gitignore:31` reservation for a secret key, and entrypoint tests.

### 3. The cli workspace: binaries, versions, descriptions

`cli/Cargo.toml`: `members = ["launcher", "kernel", "verify"]`,
`[workspace.package] version = "1.24.0-pre.7"`, `resolver = "3"`.

| Crate | Bin name(s) | Version | `package.description` |
|---|---|---|---|
| `accelerator` (launcher) | `accelerator` (`main.rs`); `accelerator-fixture` (test-only) | inherited | **none** |
| `kernel` | — (library) | inherited | **none** |
| `accelerator-verify` (verify) | `accelerator-verify` | inherited | **none** |

- **Production binaries the workspace builds today: `accelerator` +
  `accelerator-verify`.** No `accelerator-<sub>` crates. No visualiser crate.
  The manifest schema explicitly notes "Empty is valid (no external subcommands
  yet)" (`manifest.schema.json:20`).
- Every member inherits version via `version.workspace = true` — so
  `_pinned_member_versions` currently finds nothing to flag, by design.
- `.claude-plugin/plugin.json` version = `1.24.0-pre.7` (agrees).

### 4. RELEASING.md, provenance, and the retirement surface

**Runtime provenance hook to drop (doc-only, already vestigial):**
- `RELEASING.md:153-163` — "Out-of-band provenance verification" claims
  `launch-server.sh` runs `gh attestation verify` when
  `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1`. **That hook does not exist** —
  the env var and `gh attestation verify` appear nowhere in
  `skills/visualisation/visualise/scripts/launch-server.sh` (only the sha256
  check at `:125-166`). Two research notes already flag it as unimplemented
  (`2026-07-03-0164-…:240`, `2026-06-28-0136-…:628`).
- `README.md:617` (env-var table row) and `README.md:623-631` ("Provenance
  verification" section) — same stale claim. Correct all three; keep CI-side
  SLSA.

**CI-side SLSA attestation to KEEP** (`.github/workflows/main.yml`): the
`actions/attest-build-provenance@v2` steps at `:352-355` (prerelease),
`:437-440` and `:452-455` (stable + post-stable), with `attestations: write`
perms at `:334, :413`. Per ADR-0046 these stay as out-of-band provenance;
in-process SLSA is deferred.

**checksums.json retirement surface** (all one physical file,
`skills/visualisation/visualise/bin/checksums.json`, `paths.py:8`):
- Writers: `build.py:118-128` (`update_checksums_json`), `:229-238`
  (`create_checksums`), `mise.toml:123`.
- Readers: `launch-server.sh:100,125,134` (the shell launcher still consumes it —
  **coordinate with 0168**, which folds the visualiser into the workspace and
  removes the standalone `bin/checksums.json`); `build.py:58-60,141` (coherence);
  `github.py:12,140-163` (upload/verify).
- Docs/tests: `README.md:594-598`, `RELEASING.md:45,48,56,132,177`,
  `tasks/README.md:122`; `tests/conftest.py:37`, `tests/unit/tasks/test_build.py`
  (multiple), `skills/…/scripts/test-launch-server.sh:48,254,301`.
- **Ownership boundary (locked by the 0165 review):** 0165 retires the *release
  pipeline's* flat `checksums.json`; 0168 removes the *visualiser's standalone*
  `bin/checksums.json`. They are currently the same file, so the retirement and
  the visualiser-into-workspace move are entangled in practice — see Critical
  Reconciliation #1.

### 5. Release workflow structure & where signing slots in

`.github/workflows/main.yml`, three release jobs after the test/check gate:
- **`prerelease`** (`:300-360`, `macos-latest`, `if: push`): concurrency
  `accelerator-release` / `cancel-in-progress: false`; perms `id-token`,
  `contents`, `attestations: write`. Steps: checkout → mise install →
  `mise run prerelease:prepare` (`:347-350`) → attest (`:352-355`) →
  `mise run prerelease:finalise` (`:357-360`).
- **`approve-release`** (`:362-390`, `environment: release`): manual approval
  gate, deliberately no concurrency group.
- **`release`** (`:392-460`): prepare → attest → finalise for stable, then
  re-cuts the next prerelease in the same locked window.
- **Secrets today: only `GITHUB_TOKEN`** (as `GH_TOKEN`). SLSA uses OIDC, no
  long-lived secret. **No signing secret wired yet.**
- **Signing insertion point**: a new step after each `Prepare*` and
  before/alongside the `Attest*` action (between `:350↔352`, `:435↔437`,
  `:450↔452`), operating on the same binary glob; the `*:finalise` upload
  (`github.py upload_and_verify`) must additionally upload the `.minisig` files.
  A signing secret (the `-W` secret key) must be provisioned as a GHA encrypted
  secret by a repo admin — a cross-actor blocker (0165 review major #3).

## Critical Reconciliations (resolve before planning)

### #1 — The manifest binary set is empty at HEAD; 0165 ↔ 0168 sequencing

The work item says "cross-compile every workspace binary (the launcher + each
`accelerator-<sub>` sub-binary … excluding the visualiser — 0168 folds it into
the workspace and into this release)". But at current HEAD:
- The only workspace binaries are `accelerator` and `accelerator-verify`.
- Neither is an *external sub-binary*: `accelerator` is bootstrap-fetched (not
  in its own `binaries` manifest entry), and `accelerator-verify` is the trust
  shim. The manifest's `binaries` map lists **externally-dispatched
  subcommands** (ADR-0054 §L124-137) — of which there are currently **zero**.
- The visualiser (ADR-0054's "first concrete on-demand sub-binary") lives at
  `skills/visualisation/visualise/server/` and is explicitly **out of 0165's
  scope** per the work item, owned by 0168.

**Implication:** taken literally, 0165 can build all the *machinery* (multi-bin
cross-compile loop, manifest emitter, signing, coherence, upload/re-verify,
checksums retirement) but has **no non-empty manifest entry to actually ship**
until 0168 lands the visualiser into the workspace. The pipeline would emit a
valid `manifest.json` with an empty `binaries: {}`. That is *legal* per the
schema and the launcher accepts it, but it means 0165's end-to-end AC ("a
launcher built from HEAD fetches, sha256-verifies, and minisign-verifies a
pipeline-produced release end-to-end") can only be exercised against a
non-empty binary once 0168 (or a stand-in) provides one. **Decide with the
author: does 0165 ship the machinery + an empty/near-empty manifest, or is
there an implicit dependency on 0168 landing a first real sub-binary?** The
review recorded 0165 *enables* 0168 (directed), which supports "machinery
first, real binary later" — but the entangled `checksums.json` file (§4) pulls
the other way.

### #2 — "Sign whole-file (not `-H`)" vs the launcher's `allow_legacy = false`

The work item instructs: "Sign whole-file (not `-H` prehashed) to match the
launcher's whole-file `verify`." The launcher and shim both call
`verify(data, sig, /*allow_legacy=*/ false)`. In `minisign-verify`,
`allow_legacy = false` **rejects legacy (non-prehashed) signatures** and accepts
**prehashed** ones. Naively that reads as a contradiction ("sign not-prehashed"
vs "verifier requires prehashed").

**They reconcile:** modern minisign (`jedisct1/minisign` `0.12`, the pinned
version) **prehashes by default** — `minisign -S` without `-H` already emits a
prehashed signature. The work item's "not `-H`" means *don't pass the flag*
(the default already does the right thing), **not** "produce a legacy
signature." The ground truth is already in-repo: `cli/verify/tests/verify.rs`
signs with plain `minisign -S -s <secret> -x <sig> -m <target>` (no `-H`) and
asserts the `allow_legacy=false` shim verifies it. So the correct producer
invocation is **plain `minisign -S` with the `0.12` binary, no legacy flag**.
**Action for the plan:** don't take "whole-file" to mean `--legacy`; pin the
minisign version (already `0.12` in `mise.toml`) and add a round-trip test that
a pipeline-signed artifact verifies through `accelerator-verify` — this is
already an AC but the phrasing risks a wrong implementation.

### #3 — `description` source vs reality; committed key "placeholder"

- **Descriptions:** AC2 requires each binary's `description` to *equal* its
  crate's `Cargo.toml` `package.description`. **No crate carries a
  `description` today.** The manifest fixture's
  `"Launch the interactive meta-directory visualiser"` lives in the *fixture*,
  not in any Cargo.toml. So 0165 must either (a) add `package.description` to
  each shipped crate and source from it, or (b) accept there is nothing to ship
  until 0168 gives the visualiser crate a description. This is downstream of #1.
- **Committed key:** `keys/accelerator-release.pub` is a *valid* minisign key,
  not a literal placeholder — so "replace the placeholder" only entails a real
  file change if the current key's secret half is a throwaway. The plan needs
  the author to confirm whether (a) a fresh `-W` keypair is generated and the
  committed `.pub` changes (requiring the launcher be rebuilt from that HEAD so
  `build.rs` re-embeds it), or (b) the committed key is already the intended
  production key and only the secret-half provisioning (GHA secret) is missing.

## Code References

- `cli/launcher/src/launch/outbound/resolve/manifest.rs:21-120` — manifest structs,
  schema gates, sha256 sentinel, `platform_entry`.
- `cli/launcher/src/launch/outbound/resolve/verifier.rs:14-67` — sha256 + minisign
  verify order for binaries and manifest.
- `cli/launcher/src/launch/outbound/resolve/keys.rs:11-85` — embedded key,
  `TrustedKeys` verify-any-of, `allow_legacy=false` call site.
- `cli/launcher/src/launch/outbound/resolve/mod.rs:20-144` — host platform
  aliases, `expected_version`, asset naming, fetch/verify orchestration.
- `cli/launcher/build.rs:28-45` — embeds `keys/accelerator-release.pub`.
- `cli/launcher/tests/fixtures/manifest.example.json`,
  `cli/launcher/tests/fixtures/manifest.schema.json` — golden contract.
- `cli/verify/src/main.rs:24-54`, `cli/verify/tests/verify.rs:33-130` — trust
  shim + the plain-`minisign -S` round-trip proof.
- `keys/accelerator-release.pub` — committed public key (ID `001BAF670F78DDFD`).
- `cli/Cargo.toml`, `cli/{launcher,kernel,verify}/Cargo.toml` — workspace
  members, shared version, absence of `package.description`.
- `tasks/build.py:104-151,201-239` — magic-byte check, version coherence,
  cross-compile loop, checksums writer.
- `tasks/github.py:136-178` — upload/re-verify/publish + draft-preserve seam.
- `tasks/shared/{paths.py,targets.py,hashing.py}` — path locators, the four
  targets ↔ platform aliases, sha256 helper.
- `.github/workflows/main.yml:300-460` — the three release jobs + attestation
  steps + signing insertion points.
- `RELEASING.md:153-163`, `README.md:617,623-631` — stale runtime provenance
  hook to correct.
- `skills/visualisation/visualise/scripts/launch-server.sh:100,125-166` — the
  shell launcher still reading `bin/checksums.json` (0168 territory).

## Architecture Insights

- **The contract is byte-exact and asymmetric.** The consumer verifies the
  *raw bytes* of `manifest.json` against `manifest.minisig` before parsing, so
  any re-serialisation that changes bytes (key ordering, whitespace, trailing
  newline) breaks the signature. The producer must sign the *exact* bytes it
  uploads — sign the serialised file on disk, then upload that same file, never
  re-emit.
- **`sha256` is the corruption check; minisign is the trust boundary.** The
  launcher keeps both deliberately (ADR-0046: "trust rests on 'signed by our
  key', not merely 'served over TLS'"). The producer therefore emits both, with
  sha256 *inside* `manifest.json` (no sidecar sha256 assets).
- **Version coherence is the anti-rollback's producer-side twin.** The launcher
  enforces `manifest.version == its own CARGO_PKG_VERSION`; the pipeline must
  guarantee `plugin.json == workspace Cargo.toml == manifest.version` so a
  released launcher and its manifest can never disagree. The existing
  `validate_version_coherence` already spans four of the needed sources; swap
  the `checksums.json` reader for a `manifest.json` reader and the model holds.
- **cargo-dist was rejected precisely because 0165's model is per-binary,
  individually-fetched** (ADR-0046 L96-99). The hand-rolled invoke pipeline is
  the decision, not an accident — extend it, don't reach for off-the-shelf
  release tooling.
- **rustls-only is a static-linking prerequisite** (ADR-0046 L105-109):
  native-tls breaks musl-static linking. Any new crate the pipeline builds must
  stay on rustls.
- **The draft-preserve-on-failure behaviour already exists** and matches the AC
  almost exactly — 0165 extends the asset set it guards rather than inventing
  the mechanism.

## Historical Context

- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md` — the
  distribution decision: cargo-zigbuild four-target cross-compile, hand-rolled
  invoke + `gh`, sha256 + in-process minisign, SLSA deferred, musl + rustls,
  version coherence, minisign key management as an operational duty ("a leaked
  key forges releases").
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — the composition model: one `accelerator-<sub>` binary per subdomain, fetched
  on demand, manifest carries per-binary `description` for help rendering;
  `version`/`config` are built into `accelerator`; the visualiser is the *first*
  concrete sub-binary.
- `meta/reviews/work/0165-multi-binary-distribution-and-release-pipeline-review-1.md`
  — REVISE→APPROVE; locked the checksums.json ownership split (0165 vs 0168),
  the placeholder-key + HEAD-rebuild ordering, the GHA-secret provisioning
  blocker, the "no checksums.json asset" AC, the draft-preserve AC, and the
  "description == crate `Cargo.toml` description" reading.
- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md` —
  the consumer implementation research (the contract 0165 satisfies).
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — epic-level scope; flags the unimplemented runtime provenance hook and
  "keep emitting CI-side" attestation.
- `meta/plans/2026-04-30-meta-visualiser-phase-12-packaging-docs-and-release.md`
  — the current checksums.json / GitHub-Releases flow 0165 refactors away from.
- `meta/research/issues/2026-06-14-release-concurrency-group-blocks-prereleases.md`
  — the release concurrency behaviour (context for the workflow jobs).
- Luminosity reference: work item 0008 (on-demand static-binary distribution) in
  the `../luminosity` sibling repo mirrors this pipeline;
  `meta/research/codebase/2026-06-27-0157-porting-luminosity-adrs-and-feeding-spikes.md`
  records the port.

## Related Research

- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- `meta/research/codebase/2026-07-02-0163-cli-workspace-version-subcommand-scaffold.md`

## Open Questions

1. **Manifest binary set at HEAD (Reconciliation #1):** does 0165 ship the
   machinery with an empty `binaries: {}` manifest, or does it depend on 0168
   (or a stand-in) providing a first real sub-binary so the end-to-end AC can be
   exercised? What is the intended landing order given the entangled
   `checksums.json` file?
2. **Committed key (Reconciliation #3):** is a fresh `-W` keypair generated
   (changing the committed `.pub` and forcing a launcher rebuild from that HEAD),
   or is the current committed key the production key with only the secret half
   still to be provisioned?
3. **Descriptions source (Reconciliation #3):** since no crate carries
   `package.description`, does 0165 add descriptions to shipped crates, or does
   that arrive with 0168's visualiser crate?
4. **minisign `-W` in CI:** confirmed empirically that the pinned `0.12` binary
   accepts a passwordless `-W` key non-interactively in the CI runner (the C
   binary reads passwords from a TTY)? The `cli/verify` tests prove local
   round-trip; the CI path (and whether `thomasdesr/minisign-action` is adopted,
   adding a pinned supply-chain dependency) still needs confirmation.
5. **Debug archives:** the current pipeline uploads `*.debug.tar.gz` per
   platform but does not re-verify them. Are debug archives signed / listed in
   the manifest, or left as unsigned convenience assets?
