---
title: Shortcut CLI
---

`accelerator shortcut` is the sub-binary the Shortcut skills use to talk to
Shortcut's REST API v4. The skills run it for you, so you reach for it by
hand mainly to reproduce what a skill did. The
[Issue Trackers](skills/issue-trackers.mdx) page covers setup.

| Subcommand        | What it does                                                         |
|-------------------|----------------------------------------------------------------------|
| `init verify`     | Checks the token, and names its workspace, member and scopes         |
| `init list-teams` | Lists the teams the token can see, by team key                       |
| `init discover`   | Writes every team and workflow to `catalogue.json`                   |
| `show`            | Prints a story or an epic as JSON                                    |
| `search`          | Searches the base team's stories                                     |
| `create`          | Creates a story or an epic, and links the work item                  |
| `update`          | Changes named fields of a story or an epic                           |
| `comment add`     | Adds a comment to a story or an epic                                 |
| `transition`      | Moves a story to a state of its workflow                             |
| `attach`          | Adds an https link to a story                                        |

See [Internals](internals.md#terminal-invocation) for how to run
`accelerator` from a terminal.

## Keys

A Shortcut story's key is `sc-<id>`, the form Shortcut uses for branch
names. An epic's key is `epic-<id>`. Every subcommand that takes a key also
takes the web-app URL of a story or an epic in the configured workspace,
and refuses a URL for another workspace before sending anything.

`transition` and `attach` take stories only. `update --state` and
`update --estimate` also take stories only, because an epic has no workflow
state or points. Each exits 2 for an epic key.

## Output

Every subcommand prints one JSON document on stdout with a top-level
`outcome` keyword, such as `found`, `created` or `transitioned`. Failures
print an `E_*` line on stderr. For a failed write, the exit code says what a
retry would repeat: 140 when Shortcut applied nothing, 141 when it may have
applied the write, and 142 when it refused the request.

## Local development

| Mechanism                  | Purpose                                                                                                                                             |
|----------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_SHORTCUT_BIN` | One-shot override pointing `accelerator shortcut …` at a locally-built `accelerator-shortcut` binary, bypassing the normal fetch-and-cache dispatch |

Set it when working on `cli/shortcut-client/` or `cli/shortcut-cli/`, so
dispatch runs the binary you just built instead of fetching a release.
