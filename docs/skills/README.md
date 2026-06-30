# All Skills

Every user-invokable Accelerator skill, grouped by family. Each entry links to
its reference subsection on the family or concept page that homes it.

Note: `review-pr`, `review-plan`, and `review-work-item` run the multi-lens
[Review System](review-system.md); see that page for the lens catalogue.

## Development Loop

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/magnifying-glass-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/research-codebase`](development-loop.md#research-codebase) | Conduct comprehensive codebase research by<br>spawning parallel subagents and synthesising<br>findings into a research document. |
| <img src="https://api.iconify.design/ph/clipboard-text-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/create-plan`](development-loop.md#create-plan) | Create detailed implementation plans through<br>interactive, iterative collaboration. |
| <img src="https://api.iconify.design/ph/hammer-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/implement-plan`](development-loop.md#implement-plan) | Execute an approved implementation plan from<br>the configured plans directory. |
| <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/review-plan`](development-loop.md#review-plan) | Review an implementation plan through multiple<br>quality lenses and collaboratively iterate based<br>on findings. |
| <img src="https://api.iconify.design/ph/barbell-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/stress-test-plan`](development-loop.md#stress-test-plan) | Interactively stress-test an implementation plan<br>by grilling the user on decisions, edge cases,<br>and assumptions to find issues, inconsistencies,<br>and gaps before implementation begins. |
| <img src="https://api.iconify.design/ph/seal-check-bold.svg?color=%236366f1" width="20" align="center" alt=""> [`/validate-plan`](development-loop.md#validate-plan) | Validate that an implementation plan was<br>correctly executed by verifying success criteria<br>and identifying deviations. |

## Investigation & Notes

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/bug-bold.svg?color=%23f59e0b" width="20" align="center" alt=""> [`/research-issue`](investigation.md#research-issue) | Investigate production issues and bugs through<br>hypothesis-driven debugging. |
| <img src="https://api.iconify.design/ph/note-pencil-bold.svg?color=%23f59e0b" width="20" align="center" alt=""> [`/create-note`](investigation.md#create-note) | Interactively capture a short-form note. |
| <img src="https://api.iconify.design/ph/flask-bold.svg?color=%23f59e0b" width="20" align="center" alt=""> [`/conduct-spike`](investigation.md#conduct-spike) | Interactively conduct a time-boxed spike —<br>collaboratively reduce uncertainty through<br>discussion mixed with agent-driven research (and<br>small throwaway prototypes where a question is<br>empirical), then record the outcome on the<br>spike's work item. |

## Work Items

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/file-plus-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/create-work-item`](work-items.md#create-work-item) | Interactively create a well-formed work item. |
| <img src="https://api.iconify.design/ph/export-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/extract-work-items`](work-items.md#extract-work-items) | Extract work items in batch from existing<br>documents (specs, PRDs, research, plans, meeting<br>notes, design docs). |
| <img src="https://api.iconify.design/ph/sliders-horizontal-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/refine-work-item`](work-items.md#refine-work-item) | Interactively refine a work item by decomposing<br>it into children, enriching it with codebase<br>context, sharpening its acceptance criteria,<br>sizing it, or linking it to dependencies. |
| <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/review-work-item`](work-items.md#review-work-item) | Review a work item through multiple quality<br>lenses and collaboratively iterate based on<br>findings. |
| <img src="https://api.iconify.design/ph/barbell-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/stress-test-work-item`](work-items.md#stress-test-work-item) | Interactively stress-test a work item by<br>grilling the user on scope, assumptions,<br>acceptance criteria, edge cases, and dependencies<br>to surface issues, gaps, and flawed assumptions<br>before implementation is planned. |
| <img src="https://api.iconify.design/ph/pencil-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/update-work-item`](work-items.md#update-work-item) | Update fields (status, priority, tags, parent,<br>etc.) of an existing work item. |
| <img src="https://api.iconify.design/ph/list-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/list-work-items`](work-items.md#list-work-items) | List and filter work items from the configured<br>work directory. |
| <img src="https://api.iconify.design/ph/arrows-clockwise-bold.svg?color=%230d9488" width="20" align="center" alt=""> [`/sync-work-items`](work-items.md#sync-work-items) | Reconcile local work items in meta/work/ with<br>the active remote tracker named by<br>work.integration. |

## Issue Trackers

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/plug-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/init-jira`](issue-trackers.md#init-jira) | Set up the Jira Cloud integration for this<br>project. |
| <img src="https://api.iconify.design/ph/magnifying-glass-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/search-jira-issues`](issue-trackers.md#search-jira-issues) | Use this skill whenever the user wants to search,<br>list, or filter Jira tickets — by assignee,<br>status, label, project, type, component,<br>reporter, parent, or free text — even if they say<br>'find', 'show me', 'what's open', 'list my<br>tickets', or similar phrasing rather than 'search<br>Jira'. |
| <img src="https://api.iconify.design/ph/eye-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/show-jira-issue`](issue-trackers.md#show-jira-issue) | Use this skill when the user asks about a<br>specific Jira issue by key (e.g. PROJ-123,<br>ENG-456) — for viewing the description, status,<br>comments, transitions, or any other field. |
| <img src="https://api.iconify.design/ph/plus-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/create-jira-issue`](issue-trackers.md#create-jira-issue) | Use this skill only when the user explicitly<br>invokes /create-jira-issue to create a new Jira<br>issue. |
| <img src="https://api.iconify.design/ph/pencil-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/update-jira-issue`](issue-trackers.md#update-jira-issue) | Use this skill only when the user explicitly<br>invokes /update-jira-issue to modify an existing<br>Jira issue. |
| <img src="https://api.iconify.design/ph/chat-text-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/comment-jira-issue`](issue-trackers.md#comment-jira-issue) | Use this skill only when the user explicitly<br>invokes /comment-jira-issue to add, list, edit,<br>or delete comments on a Jira issue. |
| <img src="https://api.iconify.design/ph/arrows-left-right-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/transition-jira-issue`](issue-trackers.md#transition-jira-issue) | Use this skill only when the user explicitly<br>invokes /transition-jira-issue to move a Jira<br>issue through its workflow by state name. |
| <img src="https://api.iconify.design/ph/paperclip-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/attach-jira-issue`](issue-trackers.md#attach-jira-issue) | Use this skill only when the user explicitly<br>invokes /attach-jira-issue to upload one or more<br>local files as attachments to a Jira issue. |
| <img src="https://api.iconify.design/ph/plug-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/init-linear`](issue-trackers.md#init-linear) | Set up the Linear integration for this project. |
| <img src="https://api.iconify.design/ph/magnifying-glass-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/search-linear-issues`](issue-trackers.md#search-linear-issues) | Use this skill whenever the user wants to search,<br>list, or filter Linear issues — by state,<br>assignee, label, or free text — even if they say<br>'find', 'show me', 'what's open', 'list my<br>issues', or similar phrasing rather than 'search<br>Linear'. |
| <img src="https://api.iconify.design/ph/eye-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/show-linear-issue`](issue-trackers.md#show-linear-issue) | Use this skill when the user asks about a<br>specific Linear issue by identifier (e.g.<br>BLA-123, ENG-456) — for viewing the description,<br>state, assignee, or comments. |
| <img src="https://api.iconify.design/ph/plus-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/create-linear-issue`](issue-trackers.md#create-linear-issue) | Use this skill only when the user explicitly<br>invokes /create-linear-issue to create a new<br>Linear issue from a local work-item file. |
| <img src="https://api.iconify.design/ph/pencil-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/update-linear-issue`](issue-trackers.md#update-linear-issue) | Use this skill only when the user explicitly<br>invokes /update-linear-issue to change fields on<br>an existing Linear issue (title, description,<br>state, assignee, priority). |
| <img src="https://api.iconify.design/ph/chat-text-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/comment-linear-issue`](issue-trackers.md#comment-linear-issue) | Use this skill only when the user explicitly<br>invokes /comment-linear-issue to add a Markdown<br>comment to an existing Linear issue. |
| <img src="https://api.iconify.design/ph/arrows-left-right-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/transition-linear-issue`](issue-trackers.md#transition-linear-issue) | Use this skill only when the user explicitly<br>invokes /transition-linear-issue to move an<br>existing Linear issue to a different workflow<br>state. |
| <img src="https://api.iconify.design/ph/paperclip-bold.svg?color=%232563eb" width="20" align="center" alt=""> [`/attach-linear-issue`](issue-trackers.md#attach-linear-issue) | Use this skill only when the user explicitly<br>invokes /attach-linear-issue to attach a link or<br>a binary file to an existing Linear issue. |

## Architecture Decision Records

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/scroll-bold.svg?color=%237c3aed" width="20" align="center" alt=""> [`/create-adr`](adrs.md#create-adr) | Interactively create an architecture decision<br>record (ADR). |
| <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%237c3aed" width="20" align="center" alt=""> [`/review-adr`](adrs.md#review-adr) | Review an architecture decision record for<br>quality and completeness, then accept, reject, or<br>suggest revisions. |
| <img src="https://api.iconify.design/ph/export-bold.svg?color=%237c3aed" width="20" align="center" alt=""> [`/extract-adrs`](adrs.md#extract-adrs) | Extract architecture decision records from<br>existing meta documents (research, plans). |

## VCS & PR

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/git-commit-bold.svg?color=%2316a34a" width="20" align="center" alt=""> [`/commit`](vcs-and-pr.md#commit) | Create VCS commits for session changes. |
| <img src="https://api.iconify.design/ph/git-pull-request-bold.svg?color=%2316a34a" width="20" align="center" alt=""> [`/describe-pr`](vcs-and-pr.md#describe-pr) | Generate a comprehensive pull request description<br>following the repository's standard template. |
| <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%2316a34a" width="20" align="center" alt=""> [`/review-pr`](vcs-and-pr.md#review-pr) | Review a pull request through multiple quality<br>lenses and present a compiled analysis with inline<br>comments. |
| <img src="https://api.iconify.design/ph/chat-text-bold.svg?color=%2316a34a" width="20" align="center" alt=""> [`/respond-to-pr`](vcs-and-pr.md#respond-to-pr) | Respond to pull request review feedback<br>interactively, working through each item with<br>verification and code changes. |

## Design Convergence

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/swatches-bold.svg?color=%23db2777" width="20" align="center" alt=""> [`/inventory-design`](design-convergence.md#inventory-design) | Generate a structured design inventory for a<br>frontend source — tokens, components, screens, and<br>features — by crawling it with code analysis, live<br>Playwright inspection, or both. |
| <img src="https://api.iconify.design/ph/git-diff-bold.svg?color=%23db2777" width="20" align="center" alt=""> [`/analyse-design-gaps`](design-convergence.md#analyse-design-gaps) | Compare two design inventories produced by<br>inventory-design and emit a structured gap<br>artefact whose prose paragraphs satisfy the<br>extract-work-items cue-phrase contract. |

## Config & Maintenance

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/gear-six-bold.svg?color=%23475569" width="20" align="center" alt=""> [`/configure`](../configuration.md#configure) | View, create, or edit Accelerator plugin<br>configuration. |
| <img src="https://api.iconify.design/ph/rocket-launch-bold.svg?color=%23475569" width="20" align="center" alt=""> [`/init`](../configuration.md#init) | Prepare a repository with the directories and<br>gitignore entries that Accelerator skills expect. |
| <img src="https://api.iconify.design/ph/wrench-bold.svg?color=%23475569" width="20" align="center" alt=""> [`/migrate`](../migrations.md#migrate) | Apply pending Accelerator meta-directory<br>migrations to bring a repo into line with the<br>latest plugin schema. |

## Visualiser

|  |  |
| --- | --- |
| <img src="https://api.iconify.design/ph/presentation-chart-bold.svg?color=%23ea580c" width="20" align="center" alt=""> [`/visualise`](../visualiser.md#visualise) | Open the accelerator meta visualiser. |
