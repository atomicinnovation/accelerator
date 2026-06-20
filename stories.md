* Make the visualiser UI responsive and more appropriate for larger screens
* Migrate shell scripts into Rust CLI
* Graph based representation of knowledge base
* Spike incorporating graphify with an agent similar to codebase-locator
* Add configuration to visualiser frontend
* Allow selecting jj workspace / git worktree in visualiser
* Spike how to track changes to artifacts so that we can highlight change as it is happening / scan through artifact history
* Add /review-codebase command
* Add problem statement capture skill and idea generation skill
* Add competitor analysis skills
* Consider adding a refactoring workflow, driven by static analyses and code quality metrics
* Build documentation site
* Add per-user configuration of additional skill context and instructions
* Add auto-mode to all relevant skills
* Add auto-lens selection to all review skills (all, auto, manual, confirm, list)
* Consider adding effort control to review skills
* Determine a caching strategy for skills that use `token_cmd` type configuration
* Convert all artefact references to links (both in frontmatter and markdown) before rendering in visualiser
* Allow sorting by column headers in visualiser lists
* Consider adding simplicity / elegance lenses for story / plan / pr review
* Investigate how to retain edit deltas for artefacts
* Investigate how to allow comments against artefacts
* Add status mapping to work item synchronisation
* 