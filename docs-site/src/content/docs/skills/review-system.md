---
title: Review System
---

[`review-pr`](../reference/skills/github/review-pr.md),
[`review-plan`](../reference/skills/planning/review-plan.md), and
[`review-work-item`](../reference/skills/work/review-work-item.md) share
a multi-lens review system. Each lens is a specialised subagent (the
[reviewer agent](../reference/agents.md#reviewer)) that evaluates the
artefact through a specific quality perspective; the orchestrating skill
compiles the lens outputs into one analysis.

**Code review lenses** (used by `review-pr` and `review-plan`):

| Lens              | Focus                                                                |
|-------------------|----------------------------------------------------------------------|
| **Architecture**  | Modularity, coupling, dependency direction, structural drift         |
| **Code Quality**  | Complexity, design principles, error handling, code smells           |
| **Compatibility** | API contracts, cross-platform, protocol compliance, deps             |
| **Correctness**   | Logical validity, boundary conditions, state management, concurrency |
| **Database**      | Migration safety, schema design, query correctness, integrity        |
| **Documentation** | Documentation completeness, accuracy, audience fit                   |
| **Performance**   | Algorithmic efficiency, resource usage, concurrency, caching         |
| **Portability**   | Environment independence, deployment flexibility, vendor lock        |
| **Safety**        | Data loss prevention, operational safety, protective mechanisms      |
| **Security**      | OWASP Top 10, input validation, auth/authz, secrets, data flows      |
| **Standards**     | Project conventions, API standards, naming, accessibility            |
| **Test Coverage** | Coverage adequacy, assertion quality, test pyramid, anti-patterns    |
| **Usability**     | Developer experience, API ergonomics, configuration, migration paths |

**Work item review lenses** (used by `review-work-item`):

| Lens             | Focus                                                          |
|------------------|----------------------------------------------------------------|
| **Completeness** | Section presence, content density, type-appropriate content    |
| **Testability**  | Measurable criteria, verifiable outcomes, verification framing |
| **Clarity**      | Unambiguous referents, internal consistency, jargon handling   |
| **Scope**        | Right-sizing, decomposition, orthogonality                     |
| **Dependency**   | Implied couplings — blockers, consumers, external systems      |

Lenses are automatically selected based on scope, or you can specify
focus areas:

```
/review-pr 123 focus on security and architecture
/review-work-item 0042 focus on testability
```

See the [Configuration](../configuration.md) page for enabling,
disabling, or adding custom lenses.
