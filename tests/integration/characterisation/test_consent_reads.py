from dataclasses import dataclass, field

import pytest

from tests.integration.support.characterisation import Mask

PERSONAL = ".accelerator/config.local.md"


@dataclass(frozen=True)
class ConsentRead:
    key: str
    token: str
    args: tuple[str, ...]
    team: str
    personal: str
    env: dict[str, str] = field(default_factory=dict)
    needs_work_dir: bool = False


_JIRA_TEAM = (
    "---\nwork:\n  integration: jira\njira:\n  site: acme\n"
    "  email: fixture@example.com\n---\n"
)

_READS = {
    "jira-token-cmd": ConsentRead(
        "jira.token_cmd",
        "jira",
        ("fields", "list"),
        _JIRA_TEAM,
        "---\njira:\n  token_cmd: exit 3\n---\n",
    ),
    "jira-allowed-sites": ConsentRead(
        "jira.allowed_sites",
        "jira",
        ("fields", "list"),
        "---\nwork:\n  integration: jira\njira:\n"
        "  site: https://jira.example.com\n  email: fixture@example.com\n---\n",
        "---\njira:\n  allowed_sites: jira.example.com\n---\n",
        env={"ACCELERATOR_JIRA_TOKEN": "fixture-token"},
    ),
    "linear-token-cmd": ConsentRead(
        "linear.token_cmd",
        "linear",
        ("show", "X"),
        "---\nwork:\n  integration: linear\n---\n",
        "---\nlinear:\n  token_cmd: exit 3\n---\n",
    ),
    "github-token-cmd": ConsentRead(
        "github.token_cmd",
        "collaboration",
        ("pr", "base-repo", "42"),
        "---\n---\n",
        "---\ngithub:\n  token_cmd: exit 3\n---\n",
    ),
    "openalex-api-key-cmd": ConsentRead(
        "openalex.api_key_cmd",
        "research",
        ("fetch", "openalex", "search", "x"),
        "---\n---\n",
        "---\nopenalex:\n  api_key_cmd: exit 3\n---\n",
    ),
    "work-sync-jira-token-cmd": ConsentRead(
        "jira.token_cmd",
        "work",
        ("sync", "--preview"),
        _JIRA_TEAM,
        "---\njira:\n  token_cmd: exit 3\n---\n",
        needs_work_dir=True,
    ),
    "work-sync-linear-token-cmd": ConsentRead(
        "linear.token_cmd",
        "work",
        ("sync", "--preview"),
        "---\nwork:\n  integration: linear\n---\n",
        "---\nlinear:\n  token_cmd: exit 3\n---\n",
        needs_work_dir=True,
    ),
}


def _configure(repository, read_team: str, read_personal: str, tracking: str):
    repository.write(".accelerator/config.md", read_team)
    if tracking == "untracked":
        repository.untrack(PERSONAL)
    personal = repository.write(PERSONAL, read_personal)
    personal.chmod(0o600)
    if tracking == "tracked":
        repository.track(PERSONAL)
    else:
        repository.commit()


@pytest.mark.parametrize("tracking", ["tracked", "untracked"])
@pytest.mark.parametrize("read", sorted(_READS))
def test_a_root_reads_a_consent_key(
    repository, binaries, run, matches_golden, read, tracking
):
    case = _READS[read]
    if case.needs_work_dir:
        repository.write("meta/work/.keep", "")
    _configure(repository, case.team, case.personal, tracking)

    matches_golden(
        run(
            binaries.subbinary(case.token),
            *case.args,
            cwd=repository.root,
            env=case.env,
        )
    )


@pytest.mark.parametrize("tracking", ["tracked", "untracked"])
def test_design_reads_the_browser_path(
    repository,
    binaries,
    run,
    matches_golden,
    fixture_root,
    tracking,
):
    browser = fixture_root / "outside" / "chrome"
    browser.parent.mkdir()
    browser.write_text("")
    empty_bin = fixture_root / "empty-bin"
    empty_bin.mkdir()
    plugin_root = fixture_root / "plugin-root"
    plugin_root.mkdir()
    _configure(
        repository,
        "---\n---\n",
        f'---\ndesign:\n  browser_path: "{browser}"\n---\n',
        tracking,
    )

    result = run(
        binaries.subbinary("design"),
        "executor",
        "ping",
        cwd=repository.root,
        env={
            "PATH": str(empty_bin),
            "ACCELERATOR_PLUGIN_ROOT": str(plugin_root),
        },
    )
    state = repository.root / ".accelerator/tmp/inventory-design-playwright"
    browsers = sorted(p.name for p in state.iterdir()) if state.exists() else []
    matches_golden(
        result.with_observation(f"browser state: {browsers}"),
        [Mask(r"(?<=custom-)[0-9a-f]{16}", "DIGEST", literal=False)],
    )
