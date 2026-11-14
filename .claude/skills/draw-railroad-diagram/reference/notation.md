# Railroad diagram: the notation

```jsonc
{
  "id": "version-requirement",                // unique on the page it is shown on
  "kind": "railroad-diagram",
  "title": "A comparator may leave out its operator and the end of its version",   // the point
  "caption": "What to notice, with an example of what is accepted.",   // optional
  "rules": [{                                 // the rule the reader starts from first
    "name": "comparator",
    "is": [                                   // a list is a sequence
      { "optional": { "choice": ["=", ">", "<"] } },
      { "rule": "number", "status": "changed", "link": "src/parse.rs:40" },
      { "zeroOrMore": [".", { "rule": "number" }] }
    ]
  }]
}
```

## What a rule is made of

| Written as | Drawn as | Accepts |
|---|---|---|
| `"text"` or `{ "terminal": "text" }` | A pill holding the text | That text, as written |
| `{ "rule": "name" }` | A rectangle holding the name | Whatever the rule of that name accepts |
| `[a, b]` or `{ "sequence": [a, b] }` | The parts one after another along the rail | `a` then `b` |
| `{ "choice": [a, b] }` | The options stacked, the first on the line, each branching off and rejoining | Exactly one of them |
| `{ "optional": a }` | The part hung below a rail that passes it by | `a`, or nothing |
| `{ "oneOrMore": a }` | The part with a rail that loops back beneath it | `a` once or more |
| `{ "oneOrMore": a, "separator": s }` | The same, with `s` on the rail that loops back | `a`, then `s` and `a` any number of times |
| `{ "zeroOrMore": a }` | A repeat hung below a rail that passes it by; it takes a `separator` too | `a` any number of times, or nothing |

The object forms of a terminal and of a reference to a rule take `status`,
`tone` and `link`. A rule may refer to a rule the diagram does not define.

## How a rule is drawn

Its name, and under it a track that begins and ends in a pair of bars.
Rules are drawn one under another in the order written.

## Conventions

- Put the rule the reader starts from first.
- Leave as references the rules a reader already knows.
- Write a terminal exactly as it appears in the input.
- Put the commonest option of a choice first: it is the one on the line.
- Mark what the change under review did with `status`.
- Keep to about 6 rules.

## What it checks

`problemsIn` reports a diagram with no rules; a rule with no name, a name
used twice, or nothing that it is; a terminal or a reference with no text;
an empty sequence; a choice with fewer than two options; an optional or
repeated part that is missing; a part that is none of the forms above, or
more than one of them; and a `status` or `tone` outside its list.

It remarks on a rule, other than the first, that no other rule uses.

## What it leaves out

- **Wrapping** a long track onto a second line: split the rule in two.
- **Character ranges and exclusions** as forms of their own: name a rule
  for them, or write the range as a terminal's text.
- **Repeat counts** (exactly three, between two and four): say so in the
  caption.
- **Comments** on the rail.
