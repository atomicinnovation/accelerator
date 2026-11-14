# Choosing a diagram

Choose by the question the reader has, not by the subject. One subject
often raises several questions, and each gets its own diagram.

## What is there, and how is it put together?

| The question | Kind | Skill |
|---|---|---|
| How do these types relate: what extends, implements, holds or uses what? | `class-diagram` | `draw-class-diagram` |
| What does this structure look like in one concrete case? | `object-diagram` | `draw-object-diagram` |
| What is the domain made of, and where are its aggregate boundaries? | `domain-model` | `draw-domain-model` |
| Whose model sets the terms between bounded contexts? | `context-map` | `draw-context-map` |
| What are the replaceable parts, and through which interfaces do they plug together? | `component-diagram` | `draw-component-diagram` |
| How is the source organised, and which package may use which? | `package-diagram` | `draw-package-diagram` |
| What depends on what, and is there a cycle? | `dependency-graph` | `draw-dependency-graph` |
| What is the system, who uses it, and which applications and stores is it made of? | `c4-diagram` | `draw-c4-diagram` |
| What runs or is stored on which machine, process or directory? | `deployment-diagram` | `draw-deployment-diagram` |

## What happens?

| The question | Kind | Skill |
|---|---|---|
| In one scenario, who calls whom, in what order? | `sequence-diagram` | `draw-sequence-diagram` |
| What states can one thing be in, and what moves it between them? | `state-machine` | `draw-state-machine` |
| What work is done, by whom, and what runs in parallel? | `activity-diagram` | `draw-activity-diagram` |
| What are the steps of one procedure, with its loops and rejoining paths? | `flowchart` | `draw-flowchart` |
| Which conditions lead to which outcome? | `decision-tree` | `draw-decision-tree` |

## What is the data, and where does it go?

| The question | Kind | Skill |
|---|---|---|
| What is stored, how do the tables relate, and what does a migration change? | `entity-relationship-diagram` | `draw-entity-relationship-diagram` |
| Where does this data go, what keeps it, and which trust boundaries does it cross? | `data-flow-diagram` | `draw-data-flow-diagram` |
| How are the bits or bytes of this format laid out? | `byte-field-diagram` | `draw-byte-field-diagram` |
| What does this grammar, command line or pattern accept? | `railroad-diagram` | `draw-railroad-diagram` |

## When nothing above fits

| The question | Kind | Skill |
|---|---|---|
| How do these few things connect, in a way no notation above was made for? | `block-diagram` | `draw-block-diagram` |

A block diagram has no rules, so the kit can check little about it and
the reader learns only what the labels say. Reach for it last.

## Telling neighbours apart

**Sequence, activity, flowchart or state machine.** A sequence diagram is
one scenario: particular participants exchanging particular messages in
time. An activity diagram is the work itself, whoever does it, and is the
one to choose when things happen in parallel or the hand-overs between
actors matter. A flowchart is one procedure run by one actor, and is the
one to choose when the point is a loop or a branch that rejoins. A state
machine is not a procedure at all: it is everything that can happen to one
thing over its life, whatever order events arrive in.

**Flowchart or decision tree.** A decision tree when nothing loops or
rejoins and the point is the full set of cases and what each ends in. A
flowchart when steps are done along the way.

**Class diagram, object diagram or domain model.** A class diagram shows
types as the code declares them. An object diagram shows one example of
them with real values, and is the one to choose when the types are simple
but a particular arrangement is the point. A domain model shows the roles
of domain-driven design (aggregate, entity, value object, repository) and
is the one to choose when the question is what may reach what across an
aggregate boundary.

**Class diagram or component diagram.** A component diagram when the parts
are bigger than a type and the point is what plugs into what through an
interface: ports and adapters, plug-ins, services behind an API.

**Component diagram, package diagram or dependency graph.** A package
diagram is how the source is organised into namespaces. A dependency graph
is which unit depends on which at any grain (crate, module, layer), and is
the one to choose to show direction, layering or a cycle. A component
diagram is about interfaces provided and required, not merely "uses".

**C4, deployment or component.** C4 is the big picture for someone new:
people, systems and the applications and stores inside one system.
Deployment is where things physically run or rest. A component diagram
looks inside one application.

**Context map or C4 context.** A context map is about models: whose
language the other must follow, and how it protects itself. A C4 context
diagram is about users and systems and what they are for.

**Data flow or sequence.** A data flow diagram shows everywhere a kind of
data goes and rests, in no particular order, and is the one to choose for
a security review. A sequence diagram shows the order of one exchange.

**Entity-relationship, class or byte-field.** Entity-relationship for what
a database stores: tables, keys and how many at each end. A class diagram
for the same data as types in memory. A byte-field diagram when the layout
is fixed bit by bit or byte by byte.

**Block diagram or anything else.** If the arrows all mean "depends on",
it is a dependency graph; "then", a flowchart or activity diagram; "calls",
a sequence diagram; "sends data to", a data flow diagram. A block diagram
is for what is left: a derivation, a pipeline of transformations, a
handful of parts whose links are each different.

**Railroad or state machine.** A railroad diagram shows what input is
accepted. A state machine shows what a parser does as it reads.

## Views that go together

One diagram makes one point. A subject worth explaining usually needs a
structural view and a view of what happens:

| The subject | Draw |
|---|---|
| A new abstraction or port | A class or component diagram of how it fits, and a sequence diagram of it in use |
| Something with a lifecycle | A state machine of the lifecycle, and a sequence diagram of the scenario most likely to go wrong |
| A change to what is stored | An entity-relationship diagram with what changed marked, and an activity diagram or flowchart of the migration |
| Anything that handles a secret, a credential or untrusted input | A data flow diagram with its trust boundaries, and a sequence diagram of the check |
| A parser or a format | A railroad or byte-field diagram of what is accepted, and a decision tree of how what is read is classified |
| A change in how code is organised | A dependency graph, before and after |
| A new service, process or store | A C4 container diagram for where it sits, and a deployment diagram if where it runs matters |

## When not to draw

- Values across two dimensions (which case gets which outcome) read better
  as a table.
- Why a design is the way it is belongs in prose.
- A diagram with more than about a dozen elements has more than one
  point: split it by question.
