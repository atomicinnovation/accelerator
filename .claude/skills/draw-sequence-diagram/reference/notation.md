# Sequence diagram: the notation

```jsonc
{
  "id": "start-up-claims",                    // unique on the page it is shown on
  "kind": "sequence-diagram",
  "title": "Start-up claims its version before any lookup",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",         // optional
  "numbered": true,                           // optional: number the messages (default true)
  "participants": [{
    "id": "claims",                           // unique among the participants
    "label": "version_claims",                // optional: the name shown; defaults to the id
    "kind": "participant",                    // optional: "participant" (default) or "actor"
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:45"   // optional: where it is defined
  }],
  "steps": [ /* messages, fragments and notes, in the order they happen */ ]
}
```

Participants are drawn left to right in the order given. Put whoever starts
the scenario first, and each other participant in the order it is first
reached, so that most arrows point right.

## Steps

A step is a **message**, a **fragment** or a **note**.

```jsonc
// A message
{ "from": "launcher", "to": "claims",
  "label": "claim(version)",                  // what is sent: a call, an event, a signal
  "kind": "sync",                             // optional: "sync" (default), "async", "create" or "destroy"
  "reply": "Claimed",                         // optional, sync only: what comes back
  "steps": [ /* what the receiver does before it answers */ ],
  "status": "new", "tone": "risk", "link": "src/start_up.rs:78" }

// A fragment with one operand: opt, loop or break
{ "fragment": "opt", "guard": "version not cached", "steps": [ /* … */ ] }

// A fragment with several operands: alt or par
{ "fragment": "alt", "operands": [
    { "guard": "claim is fresh", "steps": [ /* … */ ] },
    { "guard": "else", "steps": [ /* … */ ] } ] }

// A note
{ "note": "Young enough: stop here.", "over": ["launcher", "claims"], "tone": "risk" }
```

**A call holds what it causes.** The `steps` of a message are what its
receiver does while handling it: the messages the receiver sends, and any
fragments and notes among them. That nesting is what places the activation
bars, so write a scenario as the call tree it is, not as a flat list.

## How it is drawn

| What | Drawn as |
|---|---|
| A participant | A box at the head of a dashed lifeline; an `actor` is a stick figure |
| A synchronous message (`sync`) | A solid line with a filled arrowhead. The receiver is active, shown as a bar on its lifeline, until it replies or has done everything in `steps` |
| A reply | A dashed line with an open arrowhead, from the receiver back to the sender, ending the receiver's bar. Written as the call's `reply`, never as a step of its own |
| An asynchronous message (`async`) | A solid line with an open arrowhead. The sender does not wait; the receiver has a bar only if the message has `steps` |
| A message to oneself | An arrow that loops out to the right and back, onto a nested bar |
| Creation (`create`) | A dashed line with an open arrowhead to the new participant's box, which is drawn there, where its life begins. Labelled `«create»` unless it has a label |
| Destruction (`destroy`) | A message ending in a cross, where the lifeline stops. Labelled `«destroy»` unless it has a label |
| `alt` | A frame divided into operands, one of which happens; each has its guard in square brackets |
| `opt` | A frame around what happens only if its guard holds |
| `loop` | A frame around what repeats; its guard says how often or for what |
| `par` | A frame divided into operands that happen at the same time |
| `break` | A frame around what happens instead of the rest of the enclosing sequence |
| A note | A sheet with a folded corner over the participants it concerns |

Messages are numbered in the order they are sent, as `3: lstat(path)`, so a
reviewer can refer to one. Replies are not numbered.

## Conventions

- Use the code's own names: a participant is a type, module, process or
  service, and a message is the function called or the event sent.
- One scenario per diagram. Draw the alternative that matters with `alt`;
  draw a different scenario as a different diagram.
- Write a `reply` when what comes back matters to the reader: a value that
  decides what happens next, or an error. Leave it out for a call that
  returns nothing of interest, and the bar simply ends.
- Mark what the change under review did with `status`, and where the
  sequence can go wrong with `tone: "risk"`.
- Keep to about 8 participants and 20 messages.

## What it checks

`problemsIn` reports a diagram with no participants or no steps; a
participant with no id, an id used twice or an unknown kind; a step that is
none of the three forms; a message naming a participant the diagram does
not have, or with no label; a kind outside the list; a reply on anything
but a synchronous call; a participant created after it has taken part, or
creating itself; any message from or to a participant after it is
destroyed; a fragment with an unknown operator, without steps, or (for
`alt` and `par`) without operands; an operand without steps; a note with no
text or over no participant; and a `status` or `tone` outside its list.

It remarks on a participant that takes no part.

## What it leaves out

- **Interaction references** (`ref`) and **gates**: a diagram cannot yet
  point to another diagram for part of its sequence.
- **Found and lost messages**: every message has a sender and a receiver
  among the participants.
- **Timing and duration constraints**, and **state invariants** on a
  lifeline.
- **The other fragment operators**: `critical`, `neg`, `assert`, `strict`,
  `seq`, `ignore` and `consider`.
- **A reply from a message to oneself**.
- **Participant stereotypes** beyond an actor: boundary, control, entity
  and database symbols.
