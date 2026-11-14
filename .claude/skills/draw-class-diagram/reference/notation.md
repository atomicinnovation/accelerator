# Class diagram: the notation

```jsonc
{
  "id": "retention-classes",                  // unique on the page it is shown on
  "kind": "class-diagram",
  "title": "Retention knows the disk and the clock only as traits",   // the point, not the topic
  "caption": "What to notice, in a sentence or two.",                 // optional
  "direction": "down",                        // optional: "down" (default) or "right"
  "classes": [{
    "id": "VersionClaim",                     // unique among the classes
    "name": "VersionClaim<'a>",               // optional: the name shown; defaults to the id
    "stereotype": "trait",                    // optional: shown as «trait» over the name
    "abstract": true,                         // optional: shown as {abstract} under the name
    "attributes": ["+ version: &'a str"],     // optional: one line each
    "operations": ["+ claim(version): Result<()>", { "text": "+ new(): Self", "static": true }],
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:24"    // optional: where the type is defined
  }],
  "relationships": [{
    "from": "VersionClaims", "to": "VersionClaim",
    "kind": "composition",                    // see the table below
    "label": "holds",                         // optional: the relationship's name
    "fromMultiplicity": "1", "toMultiplicity": "0..*",   // optional: 1, 0..1, *, 0..*, 2..5
    "fromRole": "owner", "toRole": "claims",  // optional: what each end is called at the other
    "navigable": true,                        // optional: an arrowhead at "to", for "can be reached from"
    "status": "new", "tone": "risk",          // optional
    "link": "src/retention/version_claims.rs:20"    // optional: the line that makes the relationship
  }]
}
```

## Relationships

| Kind | Write it `from` → `to` | Drawn as | Use it when |
|---|---|---|---|
| `generalisation` | specific → general | A solid line to a hollow triangle at the general type | A subclass extends a class |
| `realisation` | implementer → interface | A dashed line to a hollow triangle at the interface | A type implements an interface or trait |
| `composition` | whole → part | A solid line with a filled diamond at the whole | The whole owns the part: the part does not outlive it and has no other owner |
| `aggregation` | whole → part | A solid line with a hollow diamond at the whole | The whole groups parts it does not own |
| `association` | either → other | A solid line; an open arrowhead at `to` if `navigable` | A type holds a reference to another |
| `dependency` | user → used | A dashed line to an open arrowhead at what is used | A type uses another without holding it: a parameter, a return type, a call |

The general type is drawn above the specific, and the interface above its
implementers, whichever is written first. A whole is drawn above its
parts, and a user above what it uses.

`fromMultiplicity`, `toMultiplicity`, `fromRole`, `toRole` and `navigable`
belong only to an association, an aggregation or a composition. A
multiplicity and a role at the same end are written together there, as
`0..* claims`.

## Classes

| What | Drawn as |
|---|---|
| A class | A box with its name in bold; under a rule, its attributes; under another, its operations. A compartment with nothing in it is left out |
| A stereotype | `«trait»`, `«interface»`, `«struct»`, `«enumeration»` or any other word, over the name |
| An abstract class | `{abstract}` under the name |
| A static member | Underlined |

Write a member as UML does, one per line:
`visibility name: Type` for an attribute and
`visibility name(parameters): Type` for an operation, where visibility is
`+` public, `-` private, `#` protected or `~` package. Use the language's
own type syntax inside. For an enumeration, list its literals as
attributes.

## Conventions

- Name types as the code does, generics and lifetimes included when they
  matter to the point.
- Show the members the point needs. A box with twenty members hides the
  three that matter.
- Mark what the change under review did with `status`, and where the
  design is fragile with `tone: "risk"`.
- Keep to about 12 types.

## What it checks

`problemsIn` reports a diagram with no classes; a class with no id or an
id used twice; an `abstract` that is not true or false; a `stereotype`
that is not text; `attributes` or `operations` that are not lists, or hold
something that is neither text nor an object with text; a relationship
with no `from`, `to` or `kind`, naming a class the diagram does not have,
of an unknown kind, or written twice; a class that specialises itself, or
generalisations and realisations that run in a ring; a multiplicity, role
or `navigable` on a relationship that has no ends to put them on; a
multiplicity not written as a number, a range or `*`; and a `status`,
`tone` or `direction` outside its list.

It remarks on a class related to nothing when the diagram has
relationships, and on a part composed into more than one whole.

## What it leaves out

- **Packages** around classes. A package diagram groups types; here they
  are all at one level.
- **Association classes**, **qualified associations** and **n-ary
  associations**.
- **Constraints** and notes attached to elements, and **property strings**
  other than `{abstract}`.
- **Italics** for abstract names: `{abstract}` says the same, and the
  diagram's fonts have no italic.
- **Nested classifiers**, **template parameter boxes**, **ports** and
  **signals**.
- **Interface lollipops**: an interface is always drawn as a box.
