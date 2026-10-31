"""The closed set of edits a scenario makes to the reference set.

Every path is relative to the set root, and an edit that finds nothing to
change raises rather than leaving the set as it was.
"""

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from tasks.eval.documents.frontmatter import with_fields


def _exactly_one(count: int, what: str, path: str) -> None:
    if count != 1:
        raise ValueError(
            f"{path} must hold exactly one {what}, but holds {count}"
        )


@dataclass(frozen=True)
class SetField:
    path: str
    field: str
    value: object

    def apply(self, set_root: Path) -> None:
        target = set_root / self.path
        target.write_text(
            with_fields(target.read_text(), **{self.field: self.value})
        )


@dataclass(frozen=True)
class Remove:
    path: str

    def apply(self, set_root: Path) -> None:
        (set_root / self.path).unlink()


@dataclass(frozen=True)
class Rename:
    path: str
    to: str

    def apply(self, set_root: Path) -> None:
        (set_root / self.path).rename(set_root / self.to)


@dataclass(frozen=True)
class DropLine:
    path: str
    starting: str

    def apply(self, set_root: Path) -> None:
        target = set_root / self.path
        lines = target.read_text().splitlines(keepends=True)
        kept = [line for line in lines if not line.startswith(self.starting)]
        _exactly_one(
            len(lines) - len(kept),
            f"line starting {self.starting!r}",
            self.path,
        )
        target.write_text("".join(kept))


@dataclass(frozen=True)
class Append:
    path: str
    text: str

    def apply(self, set_root: Path) -> None:
        target = set_root / self.path
        target.write_text(target.read_text() + self.text)


@dataclass(frozen=True)
class Replace:
    path: str
    old: str
    new: str

    def apply(self, set_root: Path) -> None:
        target = set_root / self.path
        text = target.read_text()
        _exactly_one(text.count(self.old), repr(self.old), self.path)
        target.write_text(text.replace(self.old, self.new))


type Mutation = SetField | Remove | Rename | DropLine | Append | Replace


def parse_mutation(data: Mapping[str, object]) -> Mutation:
    """Read one mutation, keyed by its verb, whose value is the path."""
    match data:
        case {"set": str(path), "field": str(field), "value": value, **rest}:
            mutation: Mutation = SetField(path, field, value)
        case {"remove": str(path), **rest}:
            mutation = Remove(path)
        case {"rename": str(path), "to": str(to), **rest}:
            mutation = Rename(path, to)
        case {"drop_line": str(path), "starting": str(starting), **rest}:
            mutation = DropLine(path, starting)
        case {"append": str(path), "text": str(text), **rest}:
            mutation = Append(path, text)
        case {"replace": str(path), "old": str(old), "new": str(new), **rest}:
            mutation = Replace(path, old, new)
        case _:
            raise ValueError(f"no mutation has the shape {dict(data)}")
    if rest:
        raise ValueError(
            f"a {type(mutation).__name__} cannot take {sorted(rest)}"
        )
    return mutation
