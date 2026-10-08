"""Guard: every ``cli/`` crate dependency obeys the injection rules.

Each workspace member declares its role under
``[package.metadata.accelerator]``, plus its context and that context's kind
where the role carries one. The lint reads declared normal and build
dependencies from ``cargo metadata`` rather than ``use`` paths, so it runs on
the stable toolchain and catches a dependency declared but never imported.
Dev-dependencies are out of scope.

Whether a domain's model is genuinely expressed in an upstream's terms is a
judgement, so review enforces it, not this lint.
"""

from collections import defaultdict
from dataclasses import dataclass
from enum import StrEnum, unique
from typing import Any, override

from invoke import Context, Exit, task

from tasks.shared.cargo_metadata import Package, workspace_packages


@unique
class Role(StrEnum):
    KERNEL = "kernel"
    DOMAIN = "domain"
    ADAPTER = "adapter"
    TECHNICAL_LIBRARY = "technical-library"
    COMPOSITION_ROOT = "composition-root"
    LAUNCHER = "launcher"
    TEST_SUPPORT = "test-support"
    BOOTSTRAP_VERIFIER = "bootstrap-verifier"

    @property
    def carries_context(self) -> bool:
        return self in (Role.DOMAIN, Role.ADAPTER)

    @property
    def is_exempt(self) -> bool:
        return self in (Role.TEST_SUPPORT, Role.BOOTSTRAP_VERIFIER)

    @property
    def is_composition_root(self) -> bool:
        return self in (Role.COMPOSITION_ROOT, Role.LAUNCHER)


@unique
class Kind(StrEnum):
    PLATFORM = "platform"
    SHARED = "shared"
    PRODUCT = "product"


@unique
class Rule(StrEnum):
    INWARD = "a domain depends only on the kernel and upstream domains"
    UPSTREAM = (
        "a context depends only on a platform context or a shared context "
        "declaring it, without a cycle"
    )
    INJECTION = (
        "an adapter depends on no other context's adapter or composition "
        "root; inject its port at a composition root"
    )
    WIRING_AT_ROOTS = "a composition root depends on no other composition root"
    LAUNCHER = (
        "the launcher depends only on the kernel, platform contexts and "
        "technical libraries"
    )
    KERNEL = "the kernel depends on no workspace crate"
    TECHNICAL_LIBRARY = (
        "a technical library depends only on the kernel and technical libraries"
    )
    VERIFIER = "only the launcher depends on the bootstrap verifier"
    TEST_SUPPORT = "test support is a dev-dependency only"
    DECLARATION = "malformed role declaration"


@dataclass(frozen=True)
class BoundedContext:
    name: str
    kind: Kind
    downstreams: frozenset[str]

    def admits(self, downstream: BoundedContext) -> bool:
        match self.kind:
            case Kind.PLATFORM:
                return True
            case Kind.SHARED:
                return downstream.name in self.downstreams
            case Kind.PRODUCT:
                return False

    def shares_adapters_with(self, downstream: BoundedContext) -> bool:
        return self.kind is Kind.SHARED and self.admits(downstream)


@dataclass(frozen=True)
class Crate:
    name: str
    role: Role
    context: BoundedContext | None = None

    def is_in(self, context: BoundedContext | None) -> bool:
        return (
            self.context is not None
            and context is not None
            and self.context.name == context.name
        )


@dataclass(frozen=True)
class Edge:
    source: str
    target: str


@dataclass(frozen=True)
class Finding:
    crate: str
    rule: Rule
    dependency: str | None = None
    detail: str | None = None

    @override
    def __str__(self) -> str:
        subject = (
            f"{self.crate} -> {self.dependency}"
            if self.dependency
            else self.crate
        )
        reason = f"{self.rule} ({self.detail})" if self.detail else self.rule
        return f"{subject}: {reason}"


@dataclass(frozen=True)
class Workspace:
    crates: dict[str, Crate]


@dataclass(frozen=True)
class _Declaration:
    crate: str
    role: Role
    context: str | None
    kind: Kind | None
    downstreams: tuple[str, ...]


_ONLY_SHARED_DOMAINS = (
    "only a shared context's domain crate declares downstreams"
)


def _malformed(crate: str, detail: str) -> Finding:
    return Finding(crate, Rule.DECLARATION, detail=detail)


def _declared_table(package: Package) -> dict[str, Any]:
    return (package.get("metadata") or {}).get("accelerator") or {}


def _declaration(package: Package) -> _Declaration | Finding:
    name = package["name"]
    table = _declared_table(package)
    role = table.get("role")
    context = table.get("context")
    kind = table.get("kind")
    downstreams = table.get("downstreams")
    if role is None:
        return _malformed(name, "no role declared")
    if role not in Role:
        return _malformed(name, f"unrecognised role '{role}'")
    role = Role(role)
    if not role.carries_context and (context or kind):
        return _malformed(name, f"role '{role}' carries no context")
    if not role.carries_context and downstreams:
        return _malformed(name, _ONLY_SHARED_DOMAINS)
    if role.carries_context and context is None:
        return _malformed(name, f"role '{role}' needs a context")
    if kind is not None and kind not in Kind:
        return _malformed(name, f"unrecognised kind '{kind}'")
    return _Declaration(
        name,
        role,
        context,
        Kind(kind) if kind else None,
        tuple(downstreams or ()),
    )


def _downstream_findings(
    name: str, owner: _Declaration, known_contexts: set[str]
) -> list[Finding]:
    if owner.kind is None:
        return [_malformed(owner.crate, f"context '{name}' declares no kind")]
    if owner.kind is not Kind.SHARED:
        return (
            [_malformed(owner.crate, _ONLY_SHARED_DOMAINS)]
            if owner.downstreams
            else []
        )
    if not owner.downstreams:
        return [
            _malformed(
                owner.crate, f"shared context '{name}' declares no downstreams"
            )
        ]
    return [
        _malformed(owner.crate, f"downstream '{downstream}' is not a context")
        for downstream in owner.downstreams
        if downstream not in known_contexts
    ]


def _kind_declared_by_domain(
    name: str,
    owner: _Declaration,
    adapters: list[_Declaration],
    known_contexts: set[str],
) -> tuple[Kind | None, list[Finding]]:
    misplaced = [
        _malformed(
            adapter.crate, f"kind belongs on context '{name}''s domain crate"
        )
        for adapter in adapters
        if adapter.kind is not None
    ]
    return owner.kind, misplaced + _downstream_findings(
        name, owner, known_contexts
    )


def _kind_declared_by_adapters(
    name: str, adapters: list[_Declaration]
) -> tuple[Kind | None, list[Finding]]:
    kinds = sorted({adapter.kind for adapter in adapters if adapter.kind})
    found = [
        _malformed(adapter.crate, f"context '{name}' declares no kind")
        for adapter in adapters
        if adapter.kind is None
    ]
    if len(kinds) > 1:
        conflict = f"context '{name}' declares conflicting kinds " + ", ".join(
            kinds
        )
        found += [
            _malformed(adapter.crate, conflict)
            for adapter in adapters
            if adapter.kind is not None
        ]
    return (kinds[0] if len(kinds) == 1 else None), found


def _resolved_context(
    name: str, members: list[_Declaration], known_contexts: set[str]
) -> tuple[BoundedContext | None, list[Finding]]:
    domains = [m for m in members if m.role is Role.DOMAIN]
    adapters = [m for m in members if m.role is Role.ADAPTER]
    if domains:
        owner = domains[0]
        kind, found = _kind_declared_by_domain(
            name, owner, adapters, known_contexts
        )
        downstreams = owner.downstreams
    else:
        kind, found = _kind_declared_by_adapters(name, adapters)
        downstreams = ()
    found += [
        _malformed(adapter.crate, _ONLY_SHARED_DOMAINS)
        for adapter in adapters
        if adapter.downstreams
    ]
    if found or kind is None:
        return None, found
    return BoundedContext(name, kind, frozenset(downstreams)), found


def declarations(packages: list[Package]) -> tuple[Workspace, list[Finding]]:
    """Resolve each member's declared role and context.

    A crate whose declaration is malformed, or whose context is, is left out
    of the workspace, so no dependency rule judges it.
    """
    malformed: list[Finding] = []
    declared: list[_Declaration] = []
    refused_contexts: set[str] = set()
    for package in packages:
        declaration = _declaration(package)
        if isinstance(declaration, Finding):
            malformed.append(declaration)
            refused_contexts.add(_declared_table(package).get("context") or "")
        else:
            declared.append(declaration)

    by_context: dict[str, list[_Declaration]] = defaultdict(list)
    for declaration in declared:
        if declaration.context is not None:
            by_context[declaration.context].append(declaration)

    contexts: dict[str, BoundedContext] = {}
    for name, members in by_context.items():
        if name in refused_contexts:
            continue
        context, found = _resolved_context(name, members, set(by_context))
        malformed += found
        if context is not None:
            contexts[name] = context

    crates = {
        d.crate: Crate(d.crate, d.role, contexts.get(d.context or ""))
        for d in declared
        if d.context is None or d.context in contexts
    }
    return Workspace(crates), malformed


def edges(packages: list[Package]) -> list[Edge]:
    """Read each normal or build dependency of one member on another."""
    members = {package["name"] for package in packages}
    return list(
        dict.fromkeys(
            Edge(package["name"], dependency["name"])
            for package in packages
            for dependency in package["dependencies"]
            if dependency["kind"] != "dev" and dependency["name"] in members
        )
    )


def _upstream(source: Crate, target: Crate) -> list[Rule]:
    if (
        source.context is None
        or target.context is None
        or target.is_in(source.context)
    ):
        return []
    return [] if target.context.admits(source.context) else [Rule.UPSTREAM]


def _shares_adapters(source: Crate, target: Crate) -> bool:
    if target.is_in(source.context):
        return True
    return (
        source.context is not None
        and target.context is not None
        and target.context.shares_adapters_with(source.context)
    )


def _adapter_rules(source: Crate, target: Crate) -> list[Rule]:
    if target.role.is_composition_root:
        return [Rule.INJECTION]
    rules = _upstream(source, target)
    if target.role is Role.ADAPTER and not _shares_adapters(source, target):
        rules.append(Rule.INJECTION)
    return rules


def _launcher_rules(target: Crate) -> list[Rule]:
    if target.role is Role.COMPOSITION_ROOT:
        return [Rule.WIRING_AT_ROOTS]
    if target.context is not None and target.context.kind is not Kind.PLATFORM:
        return [Rule.LAUNCHER]
    return []


def broken_rules(source: Crate, target: Crate) -> list[Rule]:
    """Name the rules a dependency of ``source`` on ``target`` breaks."""
    if target.role is Role.TEST_SUPPORT:
        return [Rule.TEST_SUPPORT]
    if target.role is Role.BOOTSTRAP_VERIFIER:
        return [] if source.role is Role.LAUNCHER else [Rule.VERIFIER]
    match source.role:
        case Role.KERNEL:
            return [Rule.KERNEL]
        case Role.TECHNICAL_LIBRARY:
            return (
                []
                if target.role in (Role.KERNEL, Role.TECHNICAL_LIBRARY)
                else [Rule.TECHNICAL_LIBRARY]
            )
        case Role.DOMAIN:
            return (
                _upstream(source, target)
                if target.role in (Role.KERNEL, Role.DOMAIN)
                else [Rule.INWARD]
            )
        case Role.ADAPTER:
            return _adapter_rules(source, target)
        case Role.COMPOSITION_ROOT:
            return (
                [Rule.WIRING_AT_ROOTS]
                if target.role.is_composition_root
                else []
            )
        case Role.LAUNCHER:
            return _launcher_rules(target)
        case Role.TEST_SUPPORT | Role.BOOTSTRAP_VERIFIER:
            return []


@dataclass(frozen=True)
class _Crossing:
    source: Crate
    target: Crate
    origin: BoundedContext
    destination: BoundedContext


def _platform_crossings(
    judged: list[tuple[Crate, Crate]],
) -> list[_Crossing]:
    return [
        _Crossing(source, target, source.context, target.context)
        for source, target in judged
        if source.context is not None
        and target.context is not None
        and source.context.name != target.context.name
        and source.context.kind is Kind.PLATFORM
        and target.context.kind is Kind.PLATFORM
    ]


def _reachable(graph: dict[str, set[str]], start: str) -> set[str]:
    seen = {start}
    frontier = [start]
    while frontier:
        for successor in graph[frontier.pop()] - seen:
            seen.add(successor)
            frontier.append(successor)
    return seen


def _platform_cycles(crossings: list[_Crossing]) -> list[Finding]:
    graph: dict[str, set[str]] = defaultdict(set)
    for crossing in crossings:
        graph[crossing.origin.name].add(crossing.destination.name)

    found = []
    for crossing in crossings:
        origin = crossing.origin.name
        if origin not in _reachable(graph, crossing.destination.name):
            continue
        component = sorted(
            context
            for context in _reachable(graph, origin)
            if origin in _reachable(graph, context)
        )
        found.append(
            Finding(
                crossing.source.name,
                Rule.UPSTREAM,
                crossing.target.name,
                detail="cycle among " + ", ".join(component),
            )
        )
    return found


def violations(workspace: Workspace, edges: list[Edge]) -> list[Finding]:
    """Judge every dependency between two declared crates."""
    judged = [
        (workspace.crates[edge.source], workspace.crates[edge.target])
        for edge in edges
        if edge.source in workspace.crates and edge.target in workspace.crates
    ]
    found = [
        Finding(source.name, rule, target.name)
        for source, target in judged
        if not source.role.is_exempt
        for rule in broken_rules(source, target)
    ]
    return found + _platform_cycles(_platform_crossings(judged))


def findings(packages: list[Package]) -> list[Finding]:
    """Report every declaration and dependency finding, sorted."""
    workspace, malformed = declarations(packages)
    return sorted(malformed + violations(workspace, edges(packages)), key=str)


@task
def check(context: Context) -> None:
    """Fail if any crate dependency breaks the injection rules."""
    offenders = findings(workspace_packages())
    if offenders:
        raise Exit(
            "Crate dependencies break the injection rules "
            "(see [package.metadata.accelerator] in each Cargo.toml):\n  "
            + "\n  ".join(map(str, offenders)),
            code=1,
        )
