"""Design-skill, agent, and docs structure guard.

Ports the design-structure appendix of the retired shell conformance guard — a
pure content/source scan (no compiled binary), so it homes here with the other
content scanners rather than in the launcher-provisioning conformance lane.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

INIT = REPO_ROOT / "skills/config/init/SKILL.md"
CONFIGURE = REPO_ROOT / "skills/config/configure/SKILL.md"
DOCS_INTERNALS = REPO_ROOT / "docs-site/src/content/docs/internals.md"
DOCS_CONFIGURATION = REPO_ROOT / "docs-site/src/content/docs/configuration.md"
LOC = REPO_ROOT / "agents/browser-locator.md"
ANA = REPO_ROOT / "agents/browser-analyser.md"
INVENTORY = REPO_ROOT / "skills/design/inventory-design/SKILL.md"
GAPS = REPO_ROOT / "skills/design/analyse-design-gaps/SKILL.md"
DOWNGRADE_ENUM = REPO_ROOT / "cli/design/src/runtime/downgrade.rs"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_init_lists_design_path_keys_and_directory_count() -> None:
    text = _read(INIT)
    assert "research_design_inventories" in text
    assert "research_design_gaps" in text
    expected = len(
        re.findall(r"^\*\*[A-Za-z][^*]* directory\*\*:", text, re.MULTILINE)
    )
    assert f"<!-- DIR_COUNT:{expected} -->" in text
    assert "{design inventories directory}" in text
    assert "{design gaps directory}" in text


def test_configure_paths_table_lists_design_keys() -> None:
    text = _read(CONFIGURE)
    assert "research_design_inventories" in text
    assert "research_design_gaps" in text


def test_no_skill_or_agent_uses_bare_design_key_form() -> None:
    pattern = re.compile(r"config path design_(inventories|gaps)\b")
    offenders = []
    for base in ("skills", "agents"):
        for path in (REPO_ROOT / base).rglob("*"):
            if not path.is_file():
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError, OSError:
                continue
            if pattern.search(text):
                offenders.append(path.relative_to(REPO_ROOT).as_posix())
    assert not offenders, (
        f"bare design_(inventories|gaps) call sites: {offenders}"
    )


def test_docs_list_design_directories_and_template_keys() -> None:
    internals = _read(DOCS_INTERNALS)
    assert "design-inventories/" in internals
    assert "design-gaps/" in internals
    configuration = _read(DOCS_CONFIGURATION)
    assert "design-inventory" in configuration
    assert "design-gap" in configuration


def _extract_tools(path: Path) -> str:
    lines = _read(path).splitlines()
    fences = 0
    collected: str | None = None
    in_tools = False
    for line in lines:
        if line.startswith("---"):
            fences += 1
            continue
        if fences == 2:
            break
        if fences == 1 and line.startswith("tools:"):
            collected = line
            in_tools = True
            continue
        if fences == 1 and in_tools and line.startswith("  "):
            collected = f"{collected} {line}"
            continue
        if fences == 1 and in_tools:
            in_tools = False
    if collected is None:
        return ""
    body = re.sub(r"^tools:[ \t]*", "", collected)
    body = re.sub(r"^>[ \t]*", "", body)
    items = sorted(item.strip() for item in body.split(",") if item.strip())
    return ",".join(items)


def test_browser_agents_exist_and_declare_only_bash() -> None:
    assert LOC.is_file()
    assert ANA.is_file()
    assert _extract_tools(LOC) == "Bash"
    assert _extract_tools(ANA) == "Bash"
    assert "mcp__playwright__" not in _extract_tools(LOC)
    assert "mcp__playwright__" not in _extract_tools(ANA)


def test_browser_analyser_forbids_the_executor_evaluate_payloads() -> None:
    body = _read(ANA)
    for forbidden in (
        "fetch",
        "XMLHttpRequest",
        "document.cookie",
        "localStorage",
        "sessionStorage",
        "indexedDB",
        "eval",
        "innerHTML",
        "window.open",
    ):
        assert forbidden in body, f"browser-analyser omits {forbidden}"


def test_mcp_json_does_not_exist() -> None:
    assert not (REPO_ROOT / ".claude-plugin/.mcp.json").exists()


def test_inventory_design_skill_structure() -> None:
    text = _read(INVENTORY)
    assert INVENTORY.is_file()
    assert "name: inventory-design" in text
    assert 'argument-hint: "[source-id] [location]' in text
    assert "disable-model-invocation: true" in text
    assert "--allow-internal" in text
    assert "--allow-insecure-scheme" in text
    assert "mcp__playwright__" not in text
    assert "/skills/design/inventory-design/scripts/" not in text
    assert "accelerator config context" in text
    assert "accelerator config agents" in text
    tail = "\n".join(text.splitlines()[-5:])
    assert "accelerator config instructions inventory-design" in tail
    assert "accelerator:browser-locator" in text
    assert "accelerator:browser-analyser" in text


def test_analyse_design_gaps_skill_structure() -> None:
    text = _read(GAPS)
    assert GAPS.is_file()
    assert "name: analyse-design-gaps" in text
    assert 'argument-hint: "[current-source-id] [target-source-id]"' in text
    assert "we need" in text
    assert "accelerator design audit-cue-phrases" in text
    tail = "\n".join(text.splitlines()[-5:])
    assert "accelerator config instructions analyse-design-gaps" in tail


def _valid_json(path: Path) -> bool:
    import json

    try:
        json.loads(_read(path))
    except json.JSONDecodeError, OSError:
        return False
    return True


def test_design_skill_evals_are_present_and_valid_json() -> None:
    for skill in ("inventory-design", "analyse-design-gaps"):
        evals = REPO_ROOT / f"skills/design/{skill}/evals/evals.json"
        bench = REPO_ROOT / f"skills/design/{skill}/evals/benchmark.json"
        assert evals.is_file(), f"{skill}: evals.json missing"
        assert bench.is_file(), f"{skill}: benchmark.json missing"
        assert _valid_json(evals), f"{skill}: evals.json is not valid JSON"
        assert _valid_json(bench), f"{skill}: benchmark.json is not valid JSON"


def test_browser_locator_links_contract() -> None:
    loc = _read(LOC)
    assert "accelerator design executor links" in loc
    assert "Use `pathname`" in loc
    assert "Routes come from" in loc
    assert "same_origin: true" in loc
    assert "accelerator design executor links" not in _read(ANA)


_DESIGN_REFERRERS = (
    "skills/design/inventory-design/SKILL.md",
    "skills/design/analyse-design-gaps/SKILL.md",
    "agents/browser-locator.md",
    "agents/browser-analyser.md",
)

_ROOT_TOKEN = "${CLAUDE_PLUGIN_ROOT}/"


def _without_plugin_root(reference: str) -> str:
    return reference.removeprefix(_ROOT_TOKEN)


def _design_script_paths(path: Path) -> list[str]:
    paths: set[str] = set()
    for line in _read(path).splitlines():
        if "Bash(" in line:
            continue
        for match in re.findall(
            r"\$\{CLAUDE_PLUGIN_ROOT\}/skills/design/[A-Za-z0-9_./-]+", line
        ):
            if "/scripts/" in match and not match.endswith("/"):
                paths.add(match)
    return sorted(paths)


def test_design_script_references_resolve() -> None:
    for referrer in _DESIGN_REFERRERS:
        referrer_path = REPO_ROOT / referrer
        if not referrer_path.is_file():
            continue
        for reference in _design_script_paths(referrer_path):
            relative = _without_plugin_root(reference)
            assert (REPO_ROOT / relative).is_file(), (
                f"{referrer} names a missing {relative}"
            )


def _skill_body(path: Path) -> str:
    delimiters = 0
    body_lines: list[str] = []
    for line in _read(path).splitlines():
        if line == "---" and delimiters < 2:
            delimiters += 1
            continue
        if delimiters >= 2:
            body_lines.append(line)
    return "\n".join(body_lines)


def _granted_script_prefixes(path: Path) -> list[str]:
    frontmatter: list[str] = []
    for line in _read(path).splitlines():
        frontmatter.append(line)
        if line == "---" and len(frontmatter) > 1:
            break
    prefixes: set[str] = set()
    for line in frontmatter:
        for match in re.findall(
            r"Bash\(\$\{CLAUDE_PLUGIN_ROOT\}/skills/[A-Za-z0-9_./-]+\*?\)",
            line,
        ):
            prefix = match.removeprefix("Bash(").removesuffix(")")
            prefix = prefix.removesuffix("*")
            if "/scripts/" in prefix:
                prefixes.add(prefix)
    return sorted(prefixes)


def test_design_script_grants_have_call_sites() -> None:
    for skill in (INVENTORY, GAPS):
        body = _skill_body(skill)
        for prefix in _granted_script_prefixes(skill):
            assert prefix in body, (
                f"{skill.parent.name} grants but never invokes "
                f"{_without_plugin_root(prefix)}"
            )


_REASONS_EVER = [
    "unsupported-platform",
    "loader-unresolvable",
    "glibc-too-old",
    "runtime-libraries-missing",
    "artifact-unavailable",
    "materialisation-in-progress",
    "executor-ping-failed",
    "cache-unwritable",
    "disk-floor-not-met",
    "node-missing",
    "node-too-old",
    "bootstrap-failed",
]

_DESIGN_REFERENCE_FILES = (
    "skills/design/inventory-design/evals/evals.json",
    "skills/design/inventory-design/evals/benchmark.json",
    "skills/design/inventory-design/PROTOCOL.md",
)


def _live_downgrade_reasons() -> set[str]:
    return set(re.findall(r'=> "([a-z][a-z-]*)",', _read(DOWNGRADE_ENUM)))


def test_reasons_ever_covers_the_live_downgrade_vocabulary() -> None:
    live = _live_downgrade_reasons()
    missing = sorted(live - set(_REASONS_EVER))
    assert not missing, f"REASONS_EVER must be extended with: {missing}"


def test_design_eval_and_protocol_references_resolve() -> None:
    live = _live_downgrade_reasons()
    retired = [reason for reason in _REASONS_EVER if reason not in live]
    for reference_file in _DESIGN_REFERENCE_FILES:
        path = REPO_ROOT / reference_file
        if not path.is_file():
            continue
        body = _read(path)
        for script in sorted(set(re.findall(r"[A-Za-z0-9_-]+\.sh", body))):
            found = list((REPO_ROOT / "skills/design").rglob(script))
            assert found, (
                f"{path.name} names {script}, absent under skills/design"
            )
        for reason in retired:
            assert reason not in body, (
                f"{path.name} names retired downgrade reason {reason}"
            )
