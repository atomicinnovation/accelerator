"""The work skills relay a refused state root behind a local save.

`work create --push` and `create-batch --push` report `local-save` when the
state root is refused, with a `warning: -: configure failed:` line naming the
cause. The skills must relay that cause and say sync refuses until it is
fixed, rather than call the tracker unreachable.
"""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

CREATE = REPO_ROOT / "skills/work/create-work-item/SKILL.md"
EXTRACT = REPO_ROOT / "skills/work/extract-work-items/SKILL.md"

RELAY = (
    "if stderr carries a `warning: -: configure failed:` line, relay its "
    "cause; the draft cannot be pushed or promoted until it is fixed, and "
    "sync will refuse until then"
)


def _prose(path: Path) -> str:
    return " ".join(path.read_text(encoding="utf-8").split())


def test_create_relays_the_configure_warning_on_a_local_save() -> None:
    assert RELAY in _prose(CREATE)


def test_create_confirms_a_refused_local_save_with_its_fix() -> None:
    assert "— fix <cause> before syncing" in _prose(CREATE)


def test_extract_relays_the_configure_warning_on_a_local_save() -> None:
    assert RELAY in _prose(EXTRACT)


def test_extract_names_the_refused_configuration_outcome() -> None:
    text = _prose(EXTRACT)
    assert "`draft — sync refused (configuration)`" in text
    assert "relayed once" in text
