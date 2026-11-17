"""Guards that every skill running a command which refuses an untrusted
corpus tells the user what to do about it.

``accelerator work`` commands that read or write IDs exit 4 with
``E_REKEY_REQUIRED`` over an item off ``work.id_pattern`` and with
``E_WORK_ITEM_MALFORMED`` over a file whose IDs cannot be read. A skill
running one of them without a branch for both codes strands the user on
the most likely failure after upgrading.
"""

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]

GATED_COMMANDS = (
    (REPO_ROOT / "cli/work-cli/tests/fixtures/gated-commands.txt")
    .read_text()
    .split("\n")
)
REFUSING_COMMAND = re.compile(
    r"\baccelerator work (?:"
    + "|".join(re.escape(command) for command in GATED_COMMANDS if command)
    + r")\b"
)
CODES = ("E_REKEY_REQUIRED", "E_WORK_ITEM_MALFORMED")
NAMES_A_REFUSING_COMMAND_WITHOUT_RUNNING_IT = {
    "skills/integrations/jira/create-jira-issue/SKILL.md",
}


def violations(root: Path) -> list[str]:
    return sorted(
        f"{path.relative_to(root)}: {code}"
        for path in (root / "skills").rglob("SKILL.md")
        if str(path.relative_to(root))
        not in NAMES_A_REFUSING_COMMAND_WITHOUT_RUNNING_IT
        for text in [re.sub(r"\s+", " ", path.read_text())]
        if REFUSING_COMMAND.search(text)
        for code in CODES
        if code not in text
    )


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_a_skill_running_a_refusing_command_must_name_both_codes(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "skills/a/SKILL.md",
        "Run `accelerator work\nresolve <id>`. On `E_REKEY_REQUIRED` stop.\n",
    )
    assert violations(tmp_path) == ["skills/a/SKILL.md: E_WORK_ITEM_MALFORMED"]


def test_a_skill_naming_both_codes_passes(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "skills/b/SKILL.md",
        "`accelerator work id next` may refuse with `E_REKEY_REQUIRED` or "
        "`E_WORK_ITEM_MALFORMED`.\n",
    )
    assert violations(tmp_path) == []


def test_a_skill_running_no_refusing_command_is_not_checked(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path, "skills/c/SKILL.md", "`accelerator work rekey --preview`\n"
    )
    assert violations(tmp_path) == []


def test_every_skill_running_a_refusing_command_handles_both_codes() -> None:
    assert violations(REPO_ROOT) == []
