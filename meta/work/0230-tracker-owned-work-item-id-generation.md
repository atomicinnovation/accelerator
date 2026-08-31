---
type: "work-item"
id: "0230"
title: "Tracker-Owned Work Item ID Generation"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0146"
tags: ["sync", "tracker", "id-generation"]
last_updated: "2026-08-30T14:35:09+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-760"
---
# 0230: Tracker-Owned Work Item ID Generation

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a developer whose work items live in a remote tracker, I want the tracker to own
work-item ID generation when configured, so that a work item's local `id` always
matches its remote identifier (`external_id`). Stub-create the remote issue, adopt
its identifier locally, and codify the `id`-immutability boundary: immutable once
synced, provisional-and-rewritable before first push.

## Context

Today the local `id` is minted locally and can differ from `external_id`. Making the
tracker the source of truth means obtaining the tracker ID before writing the local
file, so `id` is set once and never rewritten — which resolves the immutability
tension for the online path. The mechanism reuses the existing atomic
`work create --push` create-then-write path and its orphan-stub recovery.

## Requirements

- When tracker-owned IDs are enabled and the tracker is reachable, mint the ID by
  stub-creating the remote issue and adopting its identifier as the local `id`.
- The minting entity is singular (the keyed project/team), even when the pull scope
  is broader.
- Define and enforce the `id`-immutability boundary: immutable once synced;
  provisional pre-sync IDs may be rewritten on first push.

## Acceptance Criteria

- [ ] Given tracker-owned IDs are enabled and the tracker is reachable, when an item
      is created, then its `id` equals its `external_id`.
- [ ] Given an item has been synced, when any later operation runs, then its `id` is
      never rewritten.
- [ ] Given the offline policy decided below, when an item is created without tracker
      reachability, then that policy is applied consistently.

## Open Questions

- Offline / tracker-unreachable creation: block creation, or mint a provisional
  `key`-prefixed ID and rewrite it on first push (relaxing `id` immutability for
  unsynced items only)?

## Dependencies

- Blocked by: the layered configuration key model (sibling under 0146).
- Blocks: none.

## Technical Notes

- Reuses the atomic `work create --push` path (create remote, then write local) and
  its `pending_push` orphan-stub recovery.
- Under tracker-owned IDs, `work.key` narrows to the tracker-less and
  offline-provisional cases; synced items take their key from the tracker.

## Drafting Notes

- Largest and least urgent child; the offline open question may warrant a spike
  before implementation.

## References

- Parent: 0146
