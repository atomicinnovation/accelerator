---
title: FAQ & Troubleshooting
description: Answers to common problems — visualiser binary downloads,
  checksum failures, macOS shell errors, migration prompts, and
  configuration that is not picked up.
---

## The visualiser tries to download something — is that expected?

Yes. The visualiser server is a pre-compiled binary distributed via
GitHub Releases. On first use you will see:

```
Downloading visualiser server (first run, ~8 MB)…
```

The download is fetched over enforced HTTPS, verified against the
SHA-256 digest in the plugin's `bin/checksums.json`, and cached inside
the installed plugin's `bin/` directory — subsequent launches are
offline. Released binaries also carry a SLSA build-provenance
attestation you can verify out of band with `gh attestation verify`.

:::tip
Air-gapped or restricted network? Skip the download entirely by
pointing at a binary you provide: set the `ACCELERATOR_VISUALISER_BIN`
environment variable, or `visualiser.binary` in
`.accelerator/config.local.md`.
:::

## The visualiser download fails with "checksum mismatch"

The launcher deletes the downloaded file and refuses to install it — a
corrupted or tampered binary is never run. Retry first (the download
retries transient failures itself). If it persists:

:::caution
A persistent mismatch usually means a proxy or captive portal is
rewriting the download, or the plugin's checksum manifest is out of
step with the release. Check you can reach
`github.com/atomicinnovation/accelerator/releases` directly, then
reinstall or update the plugin so `bin/checksums.json` matches the
released version.
:::

An "unsupported platform" error means there is no released binary for
your OS/architecture — macOS and Linux on arm64/x64 are supported. A
digest of all zeros means no binary was released for this plugin
version; use `ACCELERATOR_VISUALISER_BIN` as above.

## I keep being told to run /accelerator:migrate

After a plugin update, a `SessionStart` hook compares the migrations
bundled with the plugin against the ones recorded in
`.accelerator/state/migrations-applied`. If the repo is behind, every
new session prints a reminder until you run:

```
/accelerator:migrate
```

The skill refuses to run on a dirty working tree (commit or stash
first), prints a one-line preview of each pending migration before
applying, and records what it applied. Recovery is via VCS revert. To
opt out of a specific migration, use the runner's `--skip <id>` flag —
skipped migrations stay visible in the summary. See
[Migrations](../migrations.md) for the full behaviour.

:::note
Until migrations have run, skills that read configuration refuse the
legacy `.claude/accelerator.md` layout and point you at
`/accelerator:migrate` — so a stale repo fails loudly, not subtly.
:::

## My configuration changes are not picked up

Work through these in order:

1. **Right file, right place?** Config lives at
   `.accelerator/config.md` (team) and `.accelerator/config.local.md`
   (personal), resolved from the repository root. The legacy
   `.claude/accelerator.md` location is only honoured during
   migration.
2. **Skills read config live** via the `!` preprocessor at invocation
   time — a change takes effect on the next skill run, no restart
   needed. The config *summary* injected at session start is captured
   once per session, so it can look stale even when skills see the new
   values.
3. **Parser limits.** The frontmatter parser accepts simple scalars,
   inline arrays (`[a, b]`), and at most two levels of nesting. YAML
   comments are not supported — a `#` becomes part of the value — and
   unclosed frontmatter is ignored with a warning on stderr.
4. **Local overrides team.** If a key seems stuck, check whether
   `.accelerator/config.local.md` sets the same key — for any given
   key the local value silently wins.
5. **`jq` installed?** The session-start config detection hook needs
   `jq`; without it, detection silently does not run.

:::tip
`/accelerator:configure view` prints both files and the merged
effective settings — the fastest way to see what a skill will
actually read. See the
[configuration cookbook](configuration-cookbook.md) for known-good
recipes.
:::

## Do visualiser config changes apply immediately?

No — `visualiser.kanban_columns` and `visualiser.idle_timeout` are
read once at server boot. Restart the visualiser to pick them up.

## Where do I look next?

- [Getting started](../getting-started.md) — install and first loop.
- [Configuration](../configuration.md) — the full key reference.
- [Which skill do I need?](which-skill.md) — intent-to-skill index.
- [Releases and compatibility](../releases-and-compatibility.md) —
  version support policy.
