r"""Re-measure Claude Code's own verdict on each research-guard baseline row.

Run on each new Claude Code release, outside every mise aggregate: it spends
one headless model session per row. Each session is told to issue exactly one
Bash call under the allow rule `Bash(./probe *)`, in a scratch project whose
`./probe` is a custom script so read-only auto-approval cannot mask the
matching. A row's payload `touch`es a marker, so the marker shows whether
anything beyond the probe ran.

    uv run python -m tasks.probe.claude_permissions \\
        --claude "$(command -v claude)" --config-dir ~/.claude-probe

prints a diff against the fixture's column for that release, or the whole new
column when the release has none; `--update` writes it into the fixture.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURE = REPO_ROOT / "cli/research/tests/fixtures/claude-bash-baseline.tsv"

ALLOW_RULE = "Bash(./probe *)"
MARKER = "smuggled"
PROBE_SCRIPT = "#!/bin/sh\nexit 0\n"

# Releases from this one on ignore project settings in an untrusted folder and
# take the manual permission mode; earlier ones skip the trust dialog under
# `-p` and take the default mode.
MANUAL_MODE_SINCE = (2, 1, 281)

ESCAPES = {
    "⏎": "\n",
    "⇥": "\t",
    "␍": "\r",
    "⍽": "\u00a0",
    "␋": "\v",
    "␌": "\f",
}


@dataclass
class Fixture:
    preamble: list[str]
    columns: list[str]
    rows: list[list[str]]

    @classmethod
    def read(cls) -> Fixture:
        lines = FIXTURE.read_text(encoding="utf-8").splitlines()
        preamble = [line for line in lines if line.startswith("#")]
        body = [line for line in lines if not line.startswith("#")]
        return cls(
            preamble=preamble,
            columns=body[0].split("\t"),
            rows=[line.split("\t") for line in body[1:]],
        )

    def with_column(self, release: str, verdicts: list[str]) -> Fixture:
        if release in self.columns:
            index = self.columns.index(release)
            rows = [
                [*row[:index], verdict, *row[index + 1 :]]
                for row, verdict in zip(self.rows, verdicts, strict=True)
            ]
            return Fixture(self.preamble, self.columns, rows)
        rows = [
            [row[0], verdict, *row[1:]]
            for row, verdict in zip(self.rows, verdicts, strict=True)
        ]
        columns = [self.columns[0], release, *self.columns[1:]]
        return Fixture(self.preamble, columns, rows)

    def write(self) -> None:
        body = ["\t".join(self.columns), *("\t".join(r) for r in self.rows)]
        FIXTURE.write_text(
            "\n".join([*self.preamble, *body]) + "\n", encoding="utf-8"
        )


def unescape(text: str) -> str:
    for symbol, character in ESCAPES.items():
        text = text.replace(symbol, character)
    return text


def release_of(claude: Path) -> str:
    output = subprocess.run(
        [str(claude), "--version"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    found = re.search(r"\d+\.\d+\.\d+", output)
    if found is None:
        raise SystemExit(f"could not read a version from {output!r}")
    return found.group(0)


def permission_flags(release: str) -> list[str]:
    version = tuple(int(part) for part in release.split("."))
    if version >= MANUAL_MODE_SINCE:
        return [
            "--permission-mode",
            "manual",
            "--permission-prompts",
            "none",
        ]
    return ["--permission-mode", "default"]


def prompt_for(command: str) -> str:
    return (
        "Issue exactly one Bash tool call and nothing else. Its command must "
        "be the following JSON string, decoded, byte for byte, with no "
        "change of any kind: "
        f"{json.dumps(command)}"
    )


def issued_commands(events: list[dict[str, object]]) -> list[str]:
    commands: list[str] = []
    for event in events:
        message = event.get("message")
        if not isinstance(message, dict):
            continue
        content = message.get("content")
        if not isinstance(content, list):
            continue
        for block in content:
            if (
                isinstance(block, dict)
                and block.get("type") == "tool_use"
                and block.get("name") == "Bash"
            ):
                tool_input = block.get("input")
                if isinstance(tool_input, dict):
                    commands.append(str(tool_input.get("command", "")))
    return commands


def was_denied(events: list[dict[str, object]]) -> bool:
    return any(
        event.get("type") == "result" and event.get("permission_denials")
        for event in events
    )


def measure(
    claude: Path, config_dir: Path, flags: list[str], command: str
) -> str:
    """One headless session's verdict: allowed, denied, or inconclusive."""
    with tempfile.TemporaryDirectory(prefix="claude-probe-") as scratch:
        project = Path(scratch)
        probe = project / "probe"
        probe.write_text(PROBE_SCRIPT)
        probe.chmod(0o755)
        settings = json.dumps({"permissions": {"allow": [ALLOW_RULE]}})
        result = subprocess.run(
            [
                str(claude),
                "-p",
                prompt_for(command),
                "--model",
                "haiku",
                "--setting-sources",
                "project",
                "--strict-mcp-config",
                "--output-format",
                "stream-json",
                "--verbose",
                "--settings",
                settings,
                *flags,
            ],
            cwd=project,
            env={**os.environ, "CLAUDE_CONFIG_DIR": str(config_dir)},
            capture_output=True,
            text=True,
            check=False,
        )
        events = [
            json.loads(line)
            for line in result.stdout.splitlines()
            if line.startswith("{")
        ]
        if (project / MARKER).exists():
            return "allowed"
        if issued_commands(events) != [command]:
            return "inconclusive"
        return "denied" if was_denied(events) else "allowed"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--claude", type=Path, required=True)
    parser.add_argument("--config-dir", type=Path, required=True)
    parser.add_argument(
        "--update",
        action="store_true",
        help="write the measured column into the fixture",
    )
    arguments = parser.parse_args()

    release = release_of(arguments.claude)
    flags = permission_flags(release)
    fixture = Fixture.read()
    verdicts = [
        measure(arguments.claude, arguments.config_dir, flags, unescape(row[0]))
        for row in fixture.rows
    ]

    recorded = (
        fixture.columns.index(release) if release in fixture.columns else None
    )
    print(f"Claude Code {release}")
    for row, verdict in zip(fixture.rows, verdicts, strict=True):
        before = row[recorded] if recorded is not None else "unmeasured"
        if before != verdict:
            print(f"{row[0]}\t{before} -> {verdict}")

    if arguments.update:
        fixture.with_column(release, verdicts).write()
        print(f"wrote the {release} column to {FIXTURE}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
