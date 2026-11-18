# State machine: the notation

```jsonc
{
  "id": "claim-lifecycle",                    // unique on the page it is shown on
  "kind": "state-machine",
  "title": "A claim protects its version until it has gone unrefreshed for the whole window",
  "caption": "What to notice, in a sentence or two.",   // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "states": [{
    "id": "fresh",                            // unique among all states, at every depth
    "label": "Fresh",                         // optional: the name shown; defaults to the id
    "kind": "state",                          // optional: "state" (default), "initial", "final" or "choice"
    "entry": "write mtime = now",             // optional: done on entering the state
    "do": "protect the version",              // optional: done for as long as it lasts
    "exit": "unlink the claim",               // optional: done on leaving it
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:45",   // optional: where the state is defined
    "states": [ /* optional: the states it holds, making it a composite state */ ]
  }],
  "transitions": [{
    "from": "fresh", "to": "ageing",
    "event": "after(CLAIM_WINDOW / 10)",      // optional: what triggers it
    "guard": "claim is valid",                // optional: what must hold; drawn in square brackets
    "action": "refresh",                      // optional: what is done on the way; drawn after a slash
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:83"    // optional: the line that makes the transition
  }]
}
```

States are laid out in the order they are written, so write them in the
order the thing passes through them, with the initial state first.

## How it is drawn

| What | Drawn as |
|---|---|
| A state | A rounded box with its name. If it has `entry`, `do` or `exit`, they are listed under a rule as `entry / …`, `do / …`, `exit / …` |
| The initial state (`initial`) | A filled disc. It has exactly one transition out, which may carry an action but no event or guard |
| A final state (`final`) | A ringed disc. Nothing leaves it |
| A choice (`choice`) | A diamond with a transition in and two or more out, each with a guard; `else` is the guard for "otherwise" |
| A composite state | A rounded frame, named over a rule, around the states it holds. It may have an initial state of its own, where it starts when entered |
| A transition | A solid arrow with an open head, labelled `event [guard] / action` with whichever parts it has. With none, it happens as soon as its source state has finished |

A transition out of a composite state leaves it from whichever state
inside is current. A transition into a composite state starts at its
initial state; a transition may also go straight to a state inside it.

## Conventions

- Name a state for what is true while it lasts (`Fresh`, `Held`,
  `Connecting`), not for what happens (`Refresh`). What happens is an
  event or an action.
- Write an event as the code names it: a function called, a message
  received, or `after(duration)` for the passage of time.
- Mark what the change under review did with `status`, and where the
  machine can go wrong with `tone: "risk"`.
- Keep to about 10 states. Group states that share a way out into a
  composite state: one transition out of the composite replaces one out of
  each.

## What it checks

`problemsIn` reports a machine with no states; a state with no id, an id
used twice or an unknown kind; an initial, final or choice state that holds
others or has activities; an activity that is not text; a region with more
than one initial state; a transition with no `from` or `to`, or naming a
state the machine does not have; a transition out of a final state or into
an initial one; an initial state without exactly one transition out, or
whose transition has an event or guard; a choice with fewer than two
transitions out or none in; and a `status`, `tone` or `direction` outside
its list.

It remarks on a machine with no initial state; a state nothing leads to; a
state with no way out that is not final; a choice with more than one branch
without a guard; and two or more transitions out of one state on the same
event when any of them has no guard.

## What it leaves out

- **Orthogonal regions**: a composite state has one region, so it cannot
  show two things varying independently.
- **History** pseudo-states, shallow and deep.
- **Fork and join**, **junction**, **entry and exit points**, and
  **terminate**.
- **Submachine states**: a state cannot refer to another diagram.
- **Internal transitions** written inside a state beyond `entry`, `do` and
  `exit`, and **deferred events**.
