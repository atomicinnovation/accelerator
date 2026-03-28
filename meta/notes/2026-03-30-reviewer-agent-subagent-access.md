# Reviewer Agent Subagent Access

## Observation

During ADR context gathering for ADR-0008, the locator and analyser subagents
(`documents-locator`, `codebase-locator`) failed because `rg` (ripgrep) was not
available in their environment. These subagents depend on Grep and Glob tools
which require `rg`.

More broadly, the generic reviewer agent currently only has access to base tools
(Read, Grep, Glob, LS). It cannot spawn subagents like `codebase-locator`,
`codebase-analyser`, `codebase-pattern-finder`, `documents-locator`, or
`documents-analyser`.

## Suggestion

Consider updating the reviewer agent definition to allow use of locator,
analyser, and pattern-finder subagents. This would let review lenses perform
deeper codebase exploration — for example, a security reviewer could use
`codebase-pattern-finder` to find similar patterns to a vulnerability, or an
architecture reviewer could use `codebase-analyser` to understand component
boundaries more thoroughly.

## Considerations

- Adding Agent tool access increases the reviewer's capability but also its
  token budget and execution time
- Subagent spawning from within a reviewer agent means nested agents — need to
  consider context isolation implications (ADR-0001)
- May be better as an optional capability that specific lenses can opt into
  rather than a blanket change to the reviewer agent
