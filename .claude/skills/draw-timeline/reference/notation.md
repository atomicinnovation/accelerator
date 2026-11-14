# Timeline: the notation

```jsonc
{
  "id": "claim-ages",                         // unique on the page it is shown on
  "kind": "timeline",
  "title": "A claim protects its version for 14 days",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",    // optional
  "scale": "linear",                          // optional: "linear" (default) or "log"
  "from": 0, "to": 2592000,                   // optional: the ends of the axis; otherwise the least and greatest value
  "ticks": [{ "at": 1209600, "label": "14 d" }],          // values in one unit; the labels name them
  "spans": [{ "from": 0, "to": 1209600, "label": "fresh", "tone": "ok", "link": "src/claims.rs:159" }],
  "marks": [{ "at": 1728000, "label": "1.24.0", "tone": "bad" }]
}
```

Every value is a number in one unit of your choosing. `tone` is `ok`,
`warn`, `bad`, `info` or `none`, and `link` opens where the value comes
from.

## How it is drawn

| Written as | Drawn as |
|---|---|
| The axis | A line the width of the drawing |
| A tick | A short stroke on the axis with its label under it |
| A mark | A dot on the axis with its label above; neighbouring labels sit at two heights |
| A span | A bar under the axis from its start to its end, labelled inside when the label fits and under the bar when it does not |
| Spans that would overlap | Bars in lanes, one under another |

A value outside `from` and `to` is drawn at the nearer end.

## Conventions

- Use a logarithmic scale when values span more than about two orders of
  magnitude, and start it above zero.
- Label ticks as a reader thinks ("14 d"), and give the constant's name to
  the span or mark it bounds.
- Use tone for what is safe and what is not, not for decoration.
- Keep to about 6 marks and 6 spans.

## What it checks

`problemsIn` reports a timeline with no spans and no marks; a `scale`
outside its list; a `from`, `to` or `at` that is not a number; a tick with
no label; a span that ends before it starts; a `tone` outside its list; a
`link` that is not text; and, on a logarithmic scale, a value of zero or
less.

## What it leaves out

- **Dates and calendars**: values are numbers on one axis.
- **Several axes** or units in one figure.
- **Events in an order with no scale**: use a sequence diagram.
