# Decision tree: the notation

```jsonc
{
  "id": "keep-or-remove",                     // unique on the page it is shown on
  "kind": "decision-tree",
  "title": "Every doubt about a tree falls on the side of keeping it",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",                    // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "root": {
    "question": "Is the tree younger than the age backstop?",
    "id": "age",                              // optional: a name of its own, for other parts of a page to refer to
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/prune.rs:40",      // optional: where the condition is tested
    "branches": [
      { "answer": "yes",
        "status": "new", "tone": "risk", "link": "src/retention/prune.rs:41",   // optional, on the answer
        "then": { "outcome": "Keep", "tone": "ok" } },
      { "answer": "no",
        "then": { "question": "Does a claim name its version?", "branches": [ /* … */ ] } }
    ]
  }
}
```

The tree is written as it is drawn: each question holds its answers, and
each answer holds what follows, which is another question or an outcome.
So a tree cannot be written with a path that joins another or loops back.

## How it is drawn

| What | Drawn as |
|---|---|
| A question | A square-cornered box with the question in it |
| An answer | An arrow from its question to what follows, labelled with the answer |
| An outcome | A box with rounded ends, with the result in it |

Questions run down the page in the order they are asked, the root at the
top.

## Conventions

- Ask questions in the order the code tests the conditions.
- Word a question so its answers are short. `Is the claim inside the
  window?` with `yes` and `no` reads better than `Check the claim` with two
  sentences.
- The same outcome may appear at the end of several paths. Word it the same
  each time, so a reader can see that the paths agree.
- Give an outcome a `tone`: `risk` for what cannot be undone or goes wrong,
  `ok` for the safe result.
- Mark what the change under review added with `status`, on questions,
  answers and outcomes alike.
- Keep to about 6 questions and 10 outcomes.

## What it checks

`problemsIn` reports a tree with no root; anything in it that is neither a
question nor an outcome, or is both; a question or outcome that is not
text; a question with no branches; a branch that is not an object, has no
answer, or has nothing following it; an `id` used twice; and a `status`,
`tone` or `direction` outside its list. A problem is named by the answers
that lead to it.

It remarks on a question with only one answer; a question with the same
answer twice; and a question whose every answer leads to the same outcome.

## What it leaves out

- **Paths that join or loop**: that is a flowchart.
- **Chance nodes, probabilities and payoffs**, as in decision analysis.
- **Checking that the answers to a question cover every case.**
