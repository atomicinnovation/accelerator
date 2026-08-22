---
type: work-item
id: "0207"
title: "Detect Encoded Credentials In Scrubbed Artefacts"
date: "2026-08-12T23:21:12+00:00"
author: Toby Clemson
producer: implement-plan
status: ready
kind: story
priority: medium
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0196"]
tags: [design, security, secrets]
last_updated: "2026-08-12T23:21:12+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-737
---

# Detect Encoded Credentials In Scrubbed Artefacts

## Context

`accelerator design scrub-secrets` refuses to let an inventory through when it
repeats the literal value of a configured `ACCELERATOR_BROWSER_*` variable. It
reports the variable's name and never its value, so the report is safe to
print, log and commit.

Matching is literal-substring only. The CLI migration widened it once, because
`ACCELERATOR_BROWSER_AUTH_HEADER` holds a whole `Name: value` pair and the
daemon splits it on the first colon — so an artefact rendering just the bearer
token, the likely leakage shape, matched nothing. The value half is now a
needle of its own.

That closes one shape and leaves the rest. The artefacts being scanned are
model-authored prose describing a browser session, where a credential is at
least as likely to appear base64-encoded (as it would in an `Authorization`
header the model transcribed), percent-encoded (from a URL), whitespace-
normalised across a line wrap, or truncated with an ellipsis in the middle of a
long token.

## We need to

Derive candidate encodings per named value rather than matching the value
alone, and add a minimum-length prefix match so a truncated token is still
caught.

The report must keep its current property: it names the variable and never the
value, whatever encoding it matched. That is what makes a failure safe to
surface to the user and to put in a log.

Both halves need care. Too few encodings and the scrubber stays a formality;
too many, or too short a prefix, and a false positive blocks a legitimate
artefact for containing a common substring — which, since the scrubber refuses
the write, is a hard stop rather than a warning.

## We need to decide

- The minimum prefix length, balanced against false positives. A short
  credential may not support a prefix match at all.
- Whether a match on an encoding is a refusal or a warning. A refusal is
  consistent with today; a warning risks the leak it exists to prevent.

## Acceptance criteria

- [ ] Base64, percent-encoding and whitespace-normalisation of each named value
      are detected
- [ ] A truncated credential is caught by a prefix match above the chosen
      minimum length
- [ ] A value too short to support a prefix match is handled explicitly rather
      than silently skipped
- [ ] The report still names only the variable, for every encoding
- [ ] A test asserts a realistic false-positive candidate is not flagged
- [ ] `mise run` exits 0
