---
type: "note"
id: "2026-10-23-adopt-a-date-time-library"
title: "Adopt a date-time library across the CLI workspace"
date: "2026-10-23T13:44:00+00:00"
author: "Toby Clemson"
producer: "create-note"
status: "captured"
topic: "Use a date time library throughout the codebase, replacing the custom functions in cli/work/src/rekey/creation_date.rs"
tags: []
revision: "0e95a54d85c430c45a2835fe8d33e4c90b53c442"
repository: "accelerator"
last_updated: "2026-10-23T13:44:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Adopt a date-time library across the CLI workspace

`cli/work/src/rekey/creation_date.rs` hand-rolls RFC 3339 parsing and
calendar arithmetic (`seconds_since_epoch`, `days_since_epoch`,
`seconds_into_day`, `split_offset`, `fixed_digits`, `is_leap`,
`days_in_month`, `days_from_civil`) to order work items by creation date.
That is roughly 100 lines of date logic the project has to own and test,
duplicating what a library already gets right.

The workspace already depends on `time` 0.3 (`parsing`, `formatting`) through
`cli/Cargo.toml`, used by `launcher`, `migrate-cli`, `corpus-adapters` and
`work-adapters`. The visualiser server separately uses `chrono` 0.4. Settling
on one library, most likely `time` since it is already the workspace
dependency, would remove the hand-rolled code and the second library.

The hand-rolled code exists because of an architectural constraint, not by
accident: `cli/pup.ron` restricts the `work` crate, which is pure domain, to
imports from `corpus`, `kernel` and `tracker`. Options:

- Permit `time` in the `work` domain rules, treating it as a value-type
  library like `std` rather than an adapter concern.
- Parse the date in `work-adapters` and hand the domain an already-parsed
  instant, so `CreationDate` wraps a typed value instead of a string.
- Expose a date value type from `kernel`, backed by `time`, for every domain
  crate to share.

The behaviour `creation_date.rs` pins in its tests must survive the swap: a
date-only value orders as midnight UTC, an offset timestamp normalises to
UTC, and a missing or unparseable date orders after every dated item.
