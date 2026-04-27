---
work_item_id: "0099"
title: "Broken Frontmatter Example"
date: "2026-01-20T10:00:00+00:00"
author: Toby Clemson
type: task
status: draft
priority: low
parent: ""
tags: []

# 0099: Broken Frontmatter Example

This file's frontmatter is intentionally unclosed — the closing `---` line is
missing. This fixture is used by eval id 19 to exercise the
unparseable-frontmatter abort path in the create-work-item skill.

## Summary

Fixture file with unclosed frontmatter for testing error handling.
