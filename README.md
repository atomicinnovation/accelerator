# Accelerator

A Claude Code plugin providing development acceleration with multi-lens code
review, implementation planning, codebase research, and git workflow automation.

## Installation

```bash
claude plugin install atomicinnovation/accelerator
```

### Development

To install from a local checkout:

```bash
claude plugin install --path /path/to/accelerator
```

## Skills

### User-Invocable Skills

All skills are invoked with the `/accelerator:` prefix:

| Skill                 | Usage                                                  | Description                                                            |
|-----------------------|--------------------------------------------------------|------------------------------------------------------------------------|
| **commit**            | `/accelerator:commit`                                  | Create git commits with well-structured, atomic commits                |
| **create-plan**       | `/accelerator:create-plan ENG-1234`                    | Create detailed implementation plans through interactive collaboration |
| **describe-pr**       | `/accelerator:describe-pr 123`                         | Generate comprehensive PR descriptions following repo templates        |
| **implement-plan**    | `/accelerator:implement-plan @meta/plans/plan.md`      | Execute an approved plan phase by phase with verification              |
| **research-codebase** | `/accelerator:research-codebase "how does auth work?"` | Conduct deep codebase research with parallel sub-agents                |
| **respond-to-pr**     | `/accelerator:respond-to-pr 123`                       | Address PR review feedback interactively with code changes             |
| **review-plan**       | `/accelerator:review-plan @meta/plans/plan.md`         | Review a plan through multiple quality lenses                          |
| **review-pr**         | `/accelerator:review-pr 123`                           | Review a PR through multiple quality lenses with inline comments       |
| **validate-plan**     | `/accelerator:validate-plan @meta/plans/plan.md`       | Verify an implementation matches its plan                              |

### Review Lenses

The `review-pr` and `review-plan` skills use a multi-lens review system. Each
lens evaluates changes through a specific quality perspective:

| Lens              | Focus                                                                |
|-------------------|----------------------------------------------------------------------|
| **Architecture**  | Modularity, coupling, dependency direction, structural drift         |
| **Code Quality**  | Complexity, design principles, error handling, code smells           |
| **Performance**   | Algorithmic efficiency, resource usage, concurrency, caching         |
| **Security**      | OWASP Top 10, input validation, auth/authz, secrets, data flows      |
| **Standards**     | Project conventions, API standards, naming, documentation            |
| **Test Coverage** | Coverage adequacy, assertion quality, test pyramid, anti-patterns    |
| **Usability**     | Developer experience, API ergonomics, configuration, migration paths |

Lenses are automatically selected based on the PR/plan scope, or you can
specify focus areas:

```
/accelerator:review-pr 123 focus on security and architecture
```

### Planning Workflow

The planning skills support a full lifecycle:

1. `/accelerator:create-plan` — Create the implementation plan interactively
2. `/accelerator:review-plan` — Review and iterate plan quality
3. `/accelerator:implement-plan` — Execute the approved plan
4. `/accelerator:validate-plan` — Verify implementation matches the plan

## Agents

| Agent                       | Description                                                  |
|-----------------------------|--------------------------------------------------------------|
| **codebase-analyser**       | Analyses implementation details of specific components       |
| **codebase-locator**        | Locates files, directories, and components by description    |
| **codebase-pattern-finder** | Finds similar implementations and usage examples             |
| **documents-analyser**      | Deep dives on research topics from meta documents            |
| **documents-locator**       | Discovers relevant documents in meta/ directory              |
| **reviewer**                | Generic review agent spawned with lens-specific instructions |
| **web-search-researcher**   | Researches external documentation and resources              |

## Meta Directory

The `meta/` directory contains historical plans and research documents from the
plugin's development. New plugin development documentation goes here; project-
specific research stays in your project's own `meta/` directory.

## License

MIT — see [LICENSE](LICENSE).
