"""Scenario files: each scenario's question, the set it meets, and its gate."""

import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, get_args

from tasks.eval.ask_and_report.mutations import Mutation, parse_mutation

type Gate = Literal["status", "freshness", "conducted", "held_back", "open"]
type Verb = Literal["ask", "report"]
type Variant = Literal["base", "equivalence", "rewrite"]
type TemplateTier = Literal["user", "config"]


@dataclass(frozen=True)
class Setup:
    """The reference set a scenario starts from, and what it changes."""

    variant: Variant
    mutations: tuple[Mutation, ...] = ()
    breadth: int | None = None
    templates: tuple[TemplateTier, ...] = ()
    skips: tuple[str, ...] = ()


@dataclass(frozen=True)
class Scenario:
    name: str
    verb: Verb
    question: str
    times: int
    setup: Setup
    gate: Gate


@dataclass(frozen=True)
class ScenarioFile:
    family: str
    title: str
    scenarios: tuple[Scenario, ...]


def _only(data: Mapping[str, object], keys: set[str], what: str) -> None:
    unknown = set(data) - keys
    if unknown:
        raise ValueError(f"a {what} cannot declare {sorted(unknown)}")


_GATES: tuple[Gate, ...] = get_args(Gate.__value__)
_VERBS: tuple[Verb, ...] = get_args(Verb.__value__)
_VARIANTS: tuple[Variant, ...] = get_args(Variant.__value__)
_TEMPLATE_TIERS: tuple[TemplateTier, ...] = get_args(TemplateTier.__value__)


def _one_of[T: str](value: object, allowed: tuple[T, ...], what: str) -> T:
    for option in allowed:
        if value == option:
            return option
    raise ValueError(f"{value!r} is not a {what}")


def _typed[T](value: object, kind: type[T], what: str) -> T:
    if not isinstance(value, kind) or isinstance(value, bool):
        raise TypeError(f"{what} must be a {kind.__name__}, not {value!r}")
    return value


def _setup(data: Mapping[str, object]) -> Setup:
    _only(
        data, {"variant", "mutations", "breadth", "templates", "skips"}, "setup"
    )
    breadth = data.get("breadth")
    return Setup(
        variant=_one_of(data["variant"], _VARIANTS, "variant"),
        mutations=tuple(
            parse_mutation(m)
            for m in _typed(data.get("mutations", []), list, "mutations")
        ),
        breadth=None if breadth is None else _typed(breadth, int, "breadth"),
        templates=tuple(
            _one_of(tier, _TEMPLATE_TIERS, "template tier")
            for tier in _typed(data.get("templates", []), list, "templates")
        ),
        skips=tuple(
            _typed(item, str, "a skipped item")
            for item in _typed(data.get("skips", []), list, "skips")
        ),
    )


def _scenario(data: Mapping[str, object]) -> Scenario:
    _only(
        data, {"name", "verb", "question", "times", "setup", "gate"}, "scenario"
    )
    return Scenario(
        name=_typed(data["name"], str, "name"),
        verb=_one_of(data["verb"], _VERBS, "verb"),
        question=_typed(data["question"], str, "question"),
        times=_typed(data["times"], int, "times"),
        setup=_setup(_typed(data["setup"], dict, "setup")),
        gate=_one_of(data["gate"], _GATES, "gate"),
    )


def load_scenario_files(directory: Path) -> list[ScenarioFile]:
    """Every scenario file in ``directory``, refusing a name used twice."""
    files = []
    seen: set[str] = set()
    for path in sorted(directory.glob("*.json")):
        data = json.loads(path.read_text())
        _only(data, {"family", "title", "scenarios"}, "scenario file")
        scenarios = tuple(_scenario(s) for s in data["scenarios"])
        for scenario in scenarios:
            if scenario.name in seen:
                raise ValueError(f"two scenarios are named {scenario.name}")
            seen.add(scenario.name)
        files.append(
            ScenarioFile(
                family=data["family"], title=data["title"], scenarios=scenarios
            )
        )
    return files
