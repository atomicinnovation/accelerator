# Sync recovery fixtures

Each directory holds one linked work item that had no baseline entry when
captured on 2026-10-13:

| Directory | Remote |
|---|---|
| `0275` | `PP-805` |
| `0276` | `PP-860` |
| `0293` | `PP-870` |
| `0294` | `PP-878` |
| `0302` | `PP-879` |

- `local.md` is the committed work item at capture time.
- `remote.json` is `accelerator linear show <remote>` projected through the
  allowlist below, so no other tracker field is committed.

## Allowlist

```bash
jq '{outcome, data: {issue: (.data.issue | {id, identifier, title,
  description, updatedAt, state: {name: .state.name}})}}'
```

Recapture with:

```bash
accelerator linear show PP-805 | jq "$ALLOWLIST" > 0275/remote.json
```

Re-applying the allowlist to a committed `remote.json` leaves it unchanged.

A capture taken with `--comments <n>`, such as
`../linear-rewrites/comment-read-back.json`, keeps each comment's body too:

```bash
jq '{outcome, data: {issue: (.data.issue | {id, identifier, title,
  description, updatedAt, state: {name: .state.name},
  comments: {nodes: [.comments.nodes[] | {body}]}})}}'
```

It too leaves a committed capture unchanged.
