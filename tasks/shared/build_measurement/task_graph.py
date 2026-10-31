import json
from collections.abc import Mapping
from dataclasses import dataclass
from types import MappingProxyType
from typing import Self

from tasks.shared.build_measurement.errors import TaskGraphError


def _string_list(entry: Mapping[str, object], key: str) -> tuple[str, ...]:
    values = entry.get(key)
    if not isinstance(values, list) or not all(
        isinstance(value, str) for value in values
    ):
        raise TaskGraphError(
            f"mise tasks --json gave a {key} that is not a list of names: "
            f"{entry!r}"
        )
    return tuple(values)


def _named_entry(entry: object) -> tuple[str, Mapping[str, object]]:
    if not isinstance(entry, dict):
        raise TaskGraphError(
            f"mise tasks --json gave a task that is not an object: {entry!r}"
        )
    name = entry.get("name")
    if not isinstance(name, str):
        raise TaskGraphError(
            f"mise tasks --json gave a task with no name: {entry!r}"
        )
    return name, entry


@dataclass(frozen=True, slots=True)
class TaskGraph:
    depends: Mapping[str, tuple[str, ...]]
    runnable: frozenset[str]

    @classmethod
    def parse(cls, mise_tasks_json: str) -> Self:
        try:
            document = json.loads(mise_tasks_json)
        except json.JSONDecodeError as error:
            raise TaskGraphError(f"mise tasks --json: {error}") from error
        if not isinstance(document, list):
            raise TaskGraphError("mise tasks --json did not give a list")
        entries = dict(_named_entry(entry) for entry in document)
        return cls(
            depends=MappingProxyType(
                {
                    name: _string_list(entry, "depends")
                    for name, entry in entries.items()
                }
            ),
            runnable=frozenset(
                name
                for name, entry in entries.items()
                if _string_list(entry, "run")
            ),
        )

    def dependencies_of(self, task: str) -> tuple[str, ...]:
        return self.depends.get(task, ())

    def is_runnable(self, task: str) -> bool:
        return task in self.runnable

    def closure(self, root: str) -> frozenset[str]:
        reached: set[str] = set()
        frontier = list(self.dependencies_of(root))
        while frontier:
            task = frontier.pop()
            if task not in reached:
                reached.add(task)
                frontier.extend(self.dependencies_of(task))
        return frozenset(reached)
