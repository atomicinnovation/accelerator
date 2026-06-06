// Per-doc-type copy and examples used in empty-state surfaces.
// Pulled into its own file so the empty-state component can stay focused on
// layout, and so the writing is easy to audit type by type.

const TYPE_COPY = {
  work: {
    purpose:  "Atomic, shippable units of work — one story per file.",
    when:     "Open one whenever a task is concrete enough to estimate.",
    examples: ["Add three-layer review pipeline", "Ship PR review agents behind a flag"],
    template: "work",
    path:     "meta/work/",
  },
  "work-reviews": {
    purpose:  "Round-by-round reviews of work-item scope and breakdown.",
    when:     "Posted automatically when a work-item review agent runs.",
    examples: ["Centralise PATH defaults · round 1", "SSE reconnect work item · round 1"],
    template: "work-review",
    path:     "meta/work-reviews/",
  },
  "design-inventories": {
    purpose:  "Captured snapshots of an existing surface, screen-by-screen.",
    when:     "Run an inventory before drafting a gap or a redesign plan.",
    examples: ["Inventory: kanban board interaction states"],
    template: "design-inventory",
    path:     "meta/design-inventories/",
  },
  "design-gaps": {
    purpose:  "Annotated diffs between a current surface and a target design.",
    when:     "Drafted after at least one matching design-inventory exists.",
    examples: ["current-app → claude-design-prototype"],
    template: "design-gap",
    path:     "meta/design-gaps/",
  },
  research: {
    purpose:  "Prior-art write-ups and exploration notes before planning.",
    when:     "Capture once you've looked at how others have solved this.",
    examples: ["SSE reconnect patterns in browsers", "Agent orchestration patterns"],
    template: "research",
    path:     "meta/research/",
  },
  plans: {
    purpose:  "Design proposals for a work item, ready for review.",
    when:     "Open when you can sketch the shape of the solution.",
    examples: ["Three-layer review system architecture"],
    template: "plan",
    path:     "meta/plans/",
  },
  "plan-reviews": {
    purpose:  "Round-by-round reviews of a plan's design.",
    when:     "Posted by reviewers (or review agents) against a plan.",
    examples: ["Plan review · round 1 — meta-visualisation"],
    template: "plan-review",
    path:     "meta/plan-reviews/",
  },
  validations: {
    purpose:  "Empirical checks that a plan's promises hold in code.",
    when:     "Run after merge to confirm the validation criteria pass.",
    examples: ["Validation: comment preservation"],
    template: "validation",
    path:     "meta/validations/",
  },
  "pr-descriptions": {
    purpose:  "Long-form PR descriptions co-located with the plan.",
    when:     "Drafted alongside the PR for non-trivial changes.",
    examples: ["feat(agents): pr-review scaffolding"],
    template: "pr-description",
    path:     "meta/pr-descriptions/",
  },
  "pr-reviews": {
    purpose:  "Round-by-round reviews of a specific PR.",
    when:     "Posted by reviewers (or review agents) against a PR.",
    examples: ["PR review · round 1 — pr-review-agents"],
    template: "pr-review",
    path:     "meta/pr-reviews/",
  },
  decisions: {
    purpose:  "Architecture Decision Records — durable, non-reversible choices.",
    when:     "Open when a decision will outlive the work that prompted it.",
    examples: ["ETag scheme uses sha256 content hash"],
    template: "adr",
    path:     "meta/decisions/",
  },
  notes: {
    purpose:  "Short hallway captures and open questions that don't warrant a full plan.",
    when:     "Drop one in whenever something is worth remembering but doesn't fit research, a plan, or a decision yet.",
    examples: [
      "Open questions on SSE reconnect",
      "Hallway chat: kanban motion",
      "Followups from validation run",
    ],
    template: "note",
    path:     "meta/notes/",
  },
};

Object.assign(window, { TYPE_COPY });
