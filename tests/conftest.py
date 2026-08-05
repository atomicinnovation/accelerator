from pathlib import Path

import pytest

_HAZARD_SKILL = """\
---
name: alpha
description: Create things interactively. Use when the user
  wants to create a thing through iterative
  collaboration.
argument-hint: "[optional thing]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)
---

# Alpha

!`${CLAUDE_PLUGIN_ROOT}/scripts/status.sh`

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/read-path.sh plans`

Reference a work item like <ID> or `<KEPT>`.

```
0001-<literal>-example
!`${CLAUDE_PLUGIN_ROOT}/scripts/fenced.sh`
```
"""

_INTERNAL_SKILL = """\
---
name: hidden
description: Resolves internal paths. Not intended for direct invocation.
user-invocable: false
---

# Hidden

Internal machinery.
"""

_PLAIN_SKILL = """\
---
name: beta
description: Do a simple thing.
disable-model-invocation: true
---

# Beta

Plain body.
"""


@pytest.fixture
def fake_repo_tree(tmp_path: Path) -> Path:
    (tmp_path / ".claude-plugin").mkdir()
    (tmp_path / ".claude-plugin/plugin.json").write_text(
        '{"name":"accelerator","version":"1.20.0",'
        '"skills":["./skills/testcat/","./skills/othercat/deep/"]}'
    )
    for rel, content in {
        "skills/testcat/alpha/SKILL.md": _HAZARD_SKILL,
        "skills/testcat/hidden/SKILL.md": _INTERNAL_SKILL,
        "skills/othercat/deep/beta/SKILL.md": _PLAIN_SKILL,
        "skills/testcat/node_modules/stray/SKILL.md": _PLAIN_SKILL,
        "skills/testcat/alpha/scripts/test-fixtures/x/SKILL.md": (_PLAIN_SKILL),
        "skills/unregistered/gamma/SKILL.md": _PLAIN_SKILL,
    }.items():
        path = tmp_path / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    cli_dir = tmp_path / "cli"
    cli_dir.mkdir()
    (cli_dir / "Cargo.toml").write_text(
        "[workspace]\n"
        'members = ["launcher", "visualiser/server"]\n\n'
        "[workspace.package]\n"
        'version = "1.20.0"\n'
    )
    launcher_dir = cli_dir / "launcher"
    launcher_dir.mkdir()
    (launcher_dir / "Cargo.toml").write_text(
        '[package]\nname = "launcher"\nversion.workspace = true\n'
    )
    server_dir = cli_dir / "visualiser/server"
    server_dir.mkdir(parents=True)
    (server_dir / "Cargo.toml").write_text(
        '[package]\nname = "accelerator-visualiser"\nversion.workspace = true\n'
    )
    return tmp_path
