"""Tests for the exec-bit invariant guard in ``tasks/lint/scripts.py``.

The guard enforces a single invariant over every shell source: a tracked
``.sh`` is executable (``0755``) iff it is **not** on ``SHELL_LIBRARIES`` (the
checked-in manifest of sourced-only libraries). Scripts under ``test-fixtures/``
are a third category (bash-run migration fixtures) and are exempt in both
directions.

Two layers, mirroring ``test_lint.py``:

* a mocked-``shell_sources`` / patched-``repo_root`` layer that builds synthetic
  ``tmp_path`` trees and exercises every branch of the guard, and
* a real-tree integrity layer that pins ``SHELL_LIBRARIES`` membership.

Construction guards (or the synthetic tests pass vacuously):

* Every synthetic source path is *materialised on disk* at its intended mode —
  ``os.access`` silently returns ``False`` for a missing path, so a test that
  only mocks the list would make every off-list path look like a missing-``+x``
  entrypoint and pass for the wrong reason.
* The stale-entry test patches ``SHELL_LIBRARIES`` to a one-element set absent
  from the source list, so the stale-entry branch is the *sole* offender rather
  than firing incidentally on real members the synthetic list omits.
"""

import os
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import scripts as lint

REPO_ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture
def ctx():
    # `MagicMock(spec=Context)` satisfies the @task first-arg isinstance check.
    return MagicMock(spec=Context)


def _materialise(repo: Path, rel: str, *, executable: bool) -> None:
    """Write ``rel`` under ``repo`` at the intended mode.

    A path the guard reads with ``os.access`` MUST exist on disk, else it reads
    as non-executable (missing) and the test passes vacuously.
    """
    p = repo / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text("#!/usr/bin/env bash\n")
    p.chmod(0o755 if executable else 0o644)


def _setup(mocker, tmp_path: Path, layout: dict[str, bool]) -> None:
    """Patch ``repo_root``/``shell_sources`` and materialise ``layout``.

    ``layout`` maps each repo-relative source path to whether it should be
    executable on disk. ``shell_sources`` returns the sorted keys.
    """
    mocker.patch.object(lint, "repo_root", return_value=tmp_path)
    mocker.patch.object(lint, "shell_sources", return_value=sorted(layout))
    for rel, executable in layout.items():
        _materialise(tmp_path, rel, executable=executable)


class TestExecBitsSynthetic:
    def test_passes_when_invariant_holds(self, ctx, mocker, tmp_path: Path):
        # An executable off-list entrypoint AND a non-executable on-list library
        # — the guard must genuinely distinguish the two, not see uniform False.
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/lib.sh"})
        )
        _setup(
            mocker,
            tmp_path,
            {"scripts/entry.sh": True, "scripts/lib.sh": False},
        )
        lint.exec_bits(ctx)  # must not raise

    def test_flags_entrypoint_missing_x(self, ctx, mocker, tmp_path: Path):
        mocker.patch.object(lint, "SHELL_LIBRARIES", frozenset())
        _setup(mocker, tmp_path, {"scripts/entry.sh": False})
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        assert "scripts/entry.sh" in str(exc.value)

    def test_flags_library_carrying_x(self, ctx, mocker, tmp_path: Path):
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/lib.sh"})
        )
        _setup(mocker, tmp_path, {"scripts/lib.sh": True})
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        assert "scripts/lib.sh" in str(exc.value)

    def test_flags_stale_library_entry(self, ctx, mocker, tmp_path: Path):
        # SHELL_LIBRARIES names a path shell_sources() does not enumerate. Every
        # other synthetic source is executable (invariant-satisfying for the
        # one-element membership), so the stale entry is the SOLE offender.
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/gone.sh"})
        )
        _setup(
            mocker,
            tmp_path,
            {"scripts/present.sh": True, "scripts/other.sh": True},
        )
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        message = str(exc.value)
        assert "scripts/gone.sh" in message
        # The stale entry must be the only offender named.
        assert "scripts/present.sh" not in message
        assert "scripts/other.sh" not in message

    def test_exempts_test_fixtures(self, ctx, mocker, tmp_path: Path):
        # A fixture at 0644 is neither entrypoint nor library — not flagged.
        mocker.patch.object(lint, "SHELL_LIBRARIES", frozenset())
        _setup(
            mocker,
            tmp_path,
            {"skills/x/test-fixtures/seed.sh": False},
        )
        lint.exec_bits(ctx)  # must not raise

    def test_fixture_exemption_scope(self, ctx, mocker, tmp_path: Path):
        # Near-miss paths (a sibling FILE and a sibling DIRECTORY whose names
        # merely contain the segment as a substring) at a VIOLATING mode (off
        # list, 0644) MUST be flagged — the exemption is segment-scoped, not a
        # substring match. A compliant mode would pass regardless, so these are
        # deliberately non-compliant.
        mocker.patch.object(lint, "SHELL_LIBRARIES", frozenset())
        _setup(
            mocker,
            tmp_path,
            {
                "scripts/test-fixtures-x.sh": False,
                "scripts/test-fixturesX/run.sh": False,
            },
        )
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        message = str(exc.value)
        assert "scripts/test-fixtures-x.sh" in message
        assert "scripts/test-fixturesX/run.sh" in message

    def test_flags_extensionless_entrypoint(self, ctx, mocker, tmp_path: Path):
        # The extensionless _EXTRA_SHELL_SOURCES entry flows through the normal
        # off-list branch; materialised at 0644 it must be flagged.
        extra = "skills/visualisation/visualise/cli/accelerator-visualiser"
        mocker.patch.object(lint, "SHELL_LIBRARIES", frozenset())
        _setup(mocker, tmp_path, {extra: False})
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        assert "accelerator-visualiser" in str(exc.value)

    def test_fail_closed_on_empty_scope(self, ctx, mocker, tmp_path: Path):
        # An empty source set means discovery broke — fail loudly, never pass.
        mocker.patch.object(lint, "repo_root", return_value=tmp_path)
        mocker.patch.object(lint, "shell_sources", return_value=[])
        with pytest.raises(Exit):
            lint.exec_bits(ctx)

    def test_offender_message_lists_each_file(
        self, ctx, mocker, tmp_path: Path
    ):
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/lib.sh"})
        )
        _setup(
            mocker,
            tmp_path,
            {
                "scripts/entry-a.sh": False,  # off-list missing +x
                "scripts/entry-b.sh": False,  # off-list missing +x
                "scripts/lib.sh": True,  # on-list carrying +x
            },
        )
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        message = str(exc.value)
        assert "scripts/entry-a.sh" in message
        assert "scripts/entry-b.sh" in message
        assert "scripts/lib.sh" in message

    def test_offender_message_is_copy_pasteable(
        self, ctx, mocker, tmp_path: Path
    ):
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/lib.sh"})
        )
        _setup(
            mocker,
            tmp_path,
            {"scripts/entry.sh": False, "scripts/lib.sh": True},
        )
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        lines = [
            ln.strip() for ln in str(exc.value).splitlines() if "chmod" in ln
        ]
        assert lines
        for line in lines:
            command = line.split("#", 1)[0].strip()
            # The runnable portion before any comment is a bare chmod call.
            assert command.startswith(("chmod +x ", "chmod -x "))
            # The per-line "then commit" reminder must survive — the working
            # copy bit alone does not satisfy CI.
            assert "commit" in line


class TestExecBitsAntiVacuous:
    def test_known_bad_file_makes_guard_fail(self, ctx, mocker, tmp_path: Path):
        # Inject a known-bad off-list non-executable file into an otherwise
        # clean tree and assert the guard fails — proving it is not a no-op.
        mocker.patch.object(
            lint, "SHELL_LIBRARIES", frozenset({"scripts/lib.sh"})
        )
        _setup(
            mocker,
            tmp_path,
            {
                "scripts/good-entry.sh": True,
                "scripts/lib.sh": False,
                "scripts/BAD.sh": False,  # off-list, missing +x
            },
        )
        with pytest.raises(Exit) as exc:
            lint.exec_bits(ctx)
        assert "scripts/BAD.sh" in str(exc.value)

    def test_real_shell_sources_is_non_empty(self):
        # The guard's scope must be non-empty over the real repo, else the
        # synthetic tests prove nothing about production.
        assert lint.shell_sources()


# The reconciled set produced by the AC1 re-derivation (each member has >=1
# source ref and zero path invocations across the shell + skill/agent/hook
# surface). Pinned here so a transcription slip in the literal turns the build
# red.
_RECONCILED_LIBRARIES = frozenset(
    {
        "scripts/fs-common.sh",
        "scripts/hash-common.sh",
        "scripts/jsonl-common.sh",
        "scripts/log-common.sh",
        "scripts/work-common.sh",
        "scripts/config-defaults.sh",
        "scripts/config-common.sh",
        "scripts/atomic-common.sh",
        "scripts/vcs-common.sh",
        "scripts/doc-type-table.sh",
        "scripts/doc-type-inference.sh",
        "scripts/frontmatter-emission-rules.sh",
        "scripts/frontmatter-fixtures.sh",
        "scripts/interactive-harness.sh",
        "scripts/interactive-protocol.sh",
        "scripts/test-helpers.sh",
        "scripts/accelerator-scaffold.sh",
        "skills/config/migrate/scripts/interactive-lib.sh",
        "skills/github/scripts/test-helpers.sh",
        "skills/visualisation/visualise/scripts/launcher-helpers.sh",
        "skills/visualisation/visualise/scripts/test-helpers.sh",
        "skills/work/scripts/work-item-common.sh",
        "skills/work/scripts/work-item-bridge-codes.sh",
        "skills/integrations/jira/scripts/jira-common.sh",
        "skills/integrations/jira/scripts/jira-auth.sh",
        "skills/integrations/jira/scripts/jira-jql.sh",
        "skills/integrations/jira/scripts/jira-body-input.sh",
        "skills/integrations/jira/scripts/jira-custom-fields.sh",
        "skills/integrations/linear/scripts/linear-common.sh",
        "skills/integrations/linear/scripts/linear-auth.sh",
    }
)

# Dual-use scripts: sourced for their functions AND invoked by path in
# production, so they are entrypoints that must stay OFF the list at 0755.
# Pinning them is the regression net for the single most error-prone
# classification.
_DUAL_USE_SCRIPTS = (
    "scripts/linkage-parser.sh",
    "skills/design/inventory-design/scripts/validate-source.sh",
    "skills/integrations/jira/scripts/jira-fields.sh",
)


class TestShellLibrariesIntegrity:
    def test_exact_membership(self):
        # Sorted set-equality pins the literal; frozenset also dedupes, so a
        # dropped or duplicated line is caught here too.
        assert lint.SHELL_LIBRARIES == _RECONCILED_LIBRARIES

    def test_every_member_is_enumerated(self):
        sources = set(lint.shell_sources())
        missing = sorted(m for m in lint.SHELL_LIBRARIES if m not in sources)
        assert not missing, f"library-list members not enumerated: {missing}"

    def test_dual_use_scripts_are_entrypoints(self):
        repo = lint.repo_root()
        for rel in _DUAL_USE_SCRIPTS:
            assert rel not in lint.SHELL_LIBRARIES, (
                f"{rel} is dual-use (also path-invoked) and must stay OFF "
                "SHELL_LIBRARIES"
            )
            assert os.access(repo / rel, os.X_OK), (
                f"{rel} is an entrypoint and must be executable on the tree"
            )
