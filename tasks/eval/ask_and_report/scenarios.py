"""Scenario files: each scenario's question, the set it meets, and its gate."""

import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Literal, get_args

from tasks.eval.ask_and_report.mutations import Mutation, parse_mutation
from tasks.eval.ask_and_report.verdicts import (
    Criterion,
    Refusal,
    Writes,
    parse_criteria,
)

type Gate = Literal["status", "freshness", "conducted", "held_back", "open"]
type Verb = Literal["ask", "report"]
type Variant = Literal["base", "equivalence", "rewrite"]
type TemplateTier = Literal["user", "config"]
type Reply = Literal["accept", "new", "reject", "confirm", "decline"]


@dataclass(frozen=True)
class Setup:
    """The reference set a scenario starts from, and what it changes."""

    variant: Variant
    mutations: tuple[Mutation, ...] = ()
    breadth: int | None = None
    templates: tuple[TemplateTier, ...] = ()
    skips: tuple[str, ...] = ()


@dataclass(frozen=True)
class Turn:
    """What one turn sends, and what it is judged against.

    ``reply`` is the text sent; the opening turn sends the verb instead.
    """

    reply: str | None
    expect: tuple[Criterion, ...]

    @property
    def writes(self) -> tuple[str, ...]:
        """The set-relative paths this turn declares it may write."""
        return next((c.paths for c in self.expect if isinstance(c, Writes)), ())


@dataclass(frozen=True)
class Scenario:
    name: str
    verb: Verb
    question: str
    times: int
    setup: Setup
    gate: Gate
    turns: tuple[Turn, ...] = ()


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


_REPLIES: tuple[Reply, ...] = get_args(Reply.__value__)


def _reply(data: object) -> str:
    if isinstance(data, dict):
        _only(data, {"rewrite"}, "reply")
        return f"rewrite {_typed(data['rewrite'], int, 'a rewrite number')}"
    return _one_of(data, _REPLIES, "reply")


def _turn(data: Mapping[str, object], opening: bool) -> Turn:
    _only(data, {"reply", "expect"}, "turn")
    if opening == ("reply" in data):
        raise ValueError(
            "only every turn after the first sends a reply, not " + str(data)
        )
    expect = parse_criteria(_typed(data["expect"], dict, "expect"))
    if not any(isinstance(c, Writes | Refusal) for c in expect):
        raise ValueError(f"a turn must declare writes or a refusal: {data}")
    return Turn(
        reply=None if opening else _reply(data["reply"]),
        expect=expect,
    )


def _scenario(data: Mapping[str, object]) -> Scenario:
    _only(
        data,
        {"name", "verb", "question", "times", "setup", "gate", "turns"},
        "scenario",
    )
    turns = _typed(data["turns"], list, "turns")
    if not turns:
        raise ValueError(f"{data['name']} has no turns")
    scenario = Scenario(
        name=_typed(data["name"], str, "name"),
        verb=_one_of(data["verb"], _VERBS, "verb"),
        question=_typed(data["question"], str, "question"),
        times=_typed(data["times"], int, "times"),
        setup=_setup(_typed(data["setup"], dict, "setup")),
        gate=_one_of(data["gate"], _GATES, "gate"),
        turns=tuple(
            _turn(_typed(turn, dict, "turn"), opening=at == 0)
            for at, turn in enumerate(turns)
        ),
    )
    judged = any(c.is_judgement for t in scenario.turns for c in t.expect)
    if judged and scenario.times != 3:
        raise ValueError(
            f"{scenario.name} judges the model, so it must run three times"
        )
    return scenario


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
