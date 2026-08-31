---
type: "work-item"
id: "0252"
title: "Federate Config Into Domain Crates"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
parent: "work-item:0276"
tags: ["config", "crates", "refactor"]
last_updated: "2026-09-05T00:00:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Reparented under epic 0276 (Rust CLI Consolidation and Hardening): post-migration evolution of the cli/ Rust workspace, gathered from the audit of work items numbered above 0136."
schema_version: 1
external_id: "PP-837"
---

# 0252: Federate Config Into Domain Crates

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Federate configuration definitions into the specific domain crates they
belong to, so each domain owns its own config schema rather than a single
central definition.

## Context

Captured in the further-ideas backlog. Config keys for many domains are
defined centrally; ownership should sit with each domain.

## Requirements

- Each domain crate declares the config it owns.
- The central config layer composes federated definitions.

## Acceptance Criteria

- [ ] A domain's config keys are declared within that domain's crate.
- [ ] The config surface still resolves all keys through one entry point.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
