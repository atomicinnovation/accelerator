---
type: "work-item"
id: "0251"
title: "Env Var Overrides In Config Crates"
date: "2026-08-31T12:11:13+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "task"
priority: "medium"
tags: ["config", "crates"]
last_updated: "2026-08-31T12:11:13+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-781"
---

# 0251: Env Var Overrides In Config Crates

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Add support for environment-variable overrides in the config crates, so
configuration values can be overridden from the environment.

## Context

Captured in the further-ideas backlog. Several dev overrides already flow
through env vars ad hoc; the config crates should model env overrides as a
first-class precedence layer.

## Requirements

- The config crates resolve an env-var override ahead of file config where
  present.

## Acceptance Criteria

- [ ] Given an env var is set for a config key, the config crates return the
  env value in preference to the file value.

## Open Questions

- What is the precedence order across personal config, team config, and env
  overrides?

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and kind may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/notes/2026-06-23-further-ideas-backlog.md`
