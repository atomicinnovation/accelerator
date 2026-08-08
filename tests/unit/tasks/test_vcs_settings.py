"""Tests for the UserSettings guard in ``tasks/lint/vcs_settings.py``.

The guard keeps ``jj_lib``'s ``UserSettings`` and ``Workspace::load`` out of
``cli/vcs-adapters``: their defaults are private to jj-lib and were discovered
one panic at a time, and the detection paths need neither.

Two layers, mirroring ``test_store_duplication.py``:

* synthetic ``tmp_path`` trees exercising every branch of the scan, including
  the comment-stripping that lets the crate *document* the prohibition, and
* a real-tree assertion that the shipped crate is clean — so a green run means
  "scanned and found nothing" rather than "scanned nothing".
"""

from pathlib import Path

from tasks.lint import vcs_settings

REPO_ROOT = Path(__file__).resolve().parents[3]


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_flags_a_user_settings_construction(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "let settings = UserSettings::from_config(config)?;\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/library.rs:1"
    ]


def test_flags_a_workspace_load_call(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "cli/vcs-adapters/src/other.rs",
        "\nWorkspace::load(&settings, root)?;\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/other.rs:2"
    ]


def test_flags_a_fully_qualified_path(tmp_path: Path) -> None:
    # The whole reason this exists rather than a cargo-pup `denied` clause:
    # RestrictImports resolves `use` paths and cannot see this.
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "jj_lib::settings::UserSettings::from_config(c);\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/library.rs:1"
    ]


def test_scans_tests_and_fixtures_too(tmp_path: Path) -> None:
    _write(tmp_path, "cli/vcs-adapters/tests/x.rs", "UserSettings::default()\n")
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/tests/x.rs:1"
    ]


def test_ignores_other_crates(tmp_path: Path) -> None:
    _write(tmp_path, "cli/corpus-adapters/src/a.rs", "UserSettings::new()\n")
    assert vcs_settings.violations(tmp_path) == []


def test_the_dirty_paths_snapshot_module_is_individually_exempt(
    tmp_path: Path,
) -> None:
    # Snapshotting genuinely cannot avoid UserSettings — unlike the
    # settings-free detection paths this guard otherwise protects.
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library/dirty_paths.rs",
        "let settings = UserSettings::from_config(config)?;\n",
    )
    assert vcs_settings.violations(tmp_path) == []


def test_the_exemption_does_not_widen_to_a_sibling_file(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library/other.rs",
        "UserSettings::new()\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/library/other.rs:1"
    ]


def test_a_line_comment_may_name_the_prohibition(tmp_path: Path) -> None:
    # Without the strip it is impossible to document *why* UserSettings is
    # avoided in the very crate the guard covers, and an implementer works
    # around the self-flagging by mangling the word.
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "// Deliberately avoids UserSettings: its defaults are private.\n"
        "let loader = factory.create(root)?;\n",
    )
    assert vcs_settings.violations(tmp_path) == []


def test_a_block_comment_may_name_the_prohibition(tmp_path: Path) -> None:
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "/* Workspace::load needs a UserSettings,\n"
        "   so the loader is used. */\n"
        "let loader = factory.create(root)?;\n",
    )
    assert vcs_settings.violations(tmp_path) == []


def test_a_block_comment_preserves_line_numbers(tmp_path: Path) -> None:
    # A reported line must still point at the real line after stripping.
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "/* one\n   two\n   three */\nUserSettings::new()\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/library.rs:4"
    ]


def test_code_after_a_block_comment_on_one_line_is_still_scanned(
    tmp_path: Path,
) -> None:
    _write(
        tmp_path,
        "cli/vcs-adapters/src/library.rs",
        "/* fine */ UserSettings::new()\n",
    )
    assert vcs_settings.violations(tmp_path) == [
        "cli/vcs-adapters/src/library.rs:1"
    ]


def test_the_shipped_crate_is_clean() -> None:
    assert vcs_settings.violations(REPO_ROOT) == []


def test_the_real_tree_scan_is_not_vacuous() -> None:
    # A guard over a directory that does not exist, or whose files it never
    # opens, would pass the assertion above while proving nothing.
    sources = list((REPO_ROOT / vcs_settings.CRATE).rglob("*.rs"))
    assert len(sources) > 3, (
        f"only {len(sources)} Rust sources found under {vcs_settings.CRATE} — "
        "has the crate moved?"
    )
