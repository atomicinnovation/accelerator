---
type: note
id: "2026-06-23-ideas-backlog"
title: "Further Ideas Backlog"
date: "2026-06-23T17:08:56+00:00"
author: "Toby Clemson"
producer: create-note
status: captured
topic: "Further ideas backlog"
tags: [backlog, ideas]
revision: "8e0345fc5fd386a3526768c5f3ed790e1b00b26d"
repository: "miscellaneous"
last_updated: "2026-06-22T23:49:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Further Ideas Backlog

Running backlog of feature, skill, and infrastructure ideas for the
Accelerator plugin.

* [Bug] Fix font size for inline code in headings
* [Bug] Render inline code in document detail page title
* [Story] Add minimum accelerator version constraint and alert user when they need to upgrade
* [Story] Improve corpus cross-references in code comments and documentation (either ban, or make them fully resolvable)
* [Story] Allow reviewer profiles so that reviews can be triggered with different lens and config presets
* [Story] Make /respond-to-pr commit options VCS specific
* [Story] Mark as migrated to latest migration on first init
* [Story] Make either configuration or templates authoritative for valid lifecycle states on meta artefacts
* [Bug] Running visualiser hits a permission block because of dynamic args in invoked command
* [Bug] `accelerator corpus update` doesn't roundtrip quotes on frontmatter values, causing the model to reject
* [Bug] `accelerator migrate` detect dirty tree when not dirty
* [Story] Set model and/or effort on skills
* [Story] Add full support for ACCELERATOR_BROWSER_AUTH_HEADER in design skills
* [Task] Normalise datetimes used in filenames etc.
* [Task] Invoke `accelerator` directly in all skills.
* [Task] Consider using a library for deamon process management
* [Task] Retire server.pid in the design commands
* [Task] Get design-automation integration tests running in CI
* [Task] Build richer document model and consolidate all document handling onto crate
* [Task] Push all config lookups down into config crates
* [Task] Add support for env var overrides in config crates
* [Task] Federate config definitions into specific domain crates
* [Task] Survey all CLIs to see what logic can be pushed into the domain crates
* [Task] Add CLI logging
* [Story] Allow work item sync skill to merge conflicting chunks
* [Task] Push work list logic into core crate
* [Story] Allow /sync-work-item to work with a single work item
* [Bug] `accelerator help` doesn't show subcommands
* [Bug] Unify help style
* [Task] Move `adr` out of the `corpus` subcommand into its own subcommand like `work`.
* [Story] Remove references to TodoWrite etc. now that it has been disabled
* [Story] Render screenshots in design inventories in visualiser
* [Task] Rename `ForeignDirt` to avoid negative connotations
* [Task] Remove negative assertion tests that are a hangover from the Bash to Rust migration
* [Task] Collapse `--discoverability-hook` and `--format=hook` into one switch
* [Task] Remove `thiserror` from the codebase
* [Task] Move all VCS kind detection into the `vcs*` crates
* [Task] Rename the `remote-projection` crate to be more tracker or work focused
* [Task] Remove all references to Bash and exit codes from the Jira and Linear client crates
* [Task] Move `Filesystem` into a shared crate and use everywhere file system access is required, along with a fake implementation
* [Task] Rationalise `Surface` / `RemoteTracker` / client specific interfaces into a single unified interface
* [Task] Move `insecure-local-ok` marker file under `.accelerator`
* [Task] Introduce domain crate to `linear` and `jira` subcommands
* [Task] Isolate calls to `gh` into a shared Python module
