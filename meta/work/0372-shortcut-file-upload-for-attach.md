---
type: "work-item"
id: "0372"
title: "Shortcut File Upload for Attach"
date: "2026-11-14T09:52:01+00:00"
author: "Tom Scott"
producer: "implement-plan"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0181"
relates_to: ["work-item:0371"]
tags: ["work-management", "integrations", "shortcut"]
last_updated: "2026-11-14T09:52:01+00:00"
last_updated_by: "Tom Scott"
schema_version: 1
---
# 0372: Shortcut File Upload for Attach

## Summary

`accelerator shortcut attach` adds https links only. This item adds file
upload, so `attach --file` works for Shortcut as it does for Jira and Linear.

## Context

Work item 0371 shipped `attach` with links only: v4's `external_links_add`
takes bare URLs, and file upload uses a separate multipart endpoint that 0371
did not build. `attach --file` exits 2 with "not supported yet".

## Acceptance Criteria

- [ ] Given a readable file, when `attach sc-<id> --file <path>` runs, then
      the file appears on the story in Shortcut.
- [ ] Given a failed upload, then nothing is linked to the story, and the
      exit code says whether a retry would repeat the upload.
- [ ] `attach-shortcut-story` offers file upload, with the same preview and
      confirm step.
