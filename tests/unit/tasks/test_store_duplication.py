"""Tests for the store-duplication guard in ``tasks/lint/store_duplication.py``.

The guard keeps the whole-file atomic-write primitive consolidated in the
``cli/store`` crate: a ``fs::rename`` / ``NamedTempFile`` / ``.persist`` shape
anywhere else under ``cli/**/src`` is a reintroduced duplicate. Two renames are
genuine non-duplicates and are allowlisted (the cache publisher, the mkdir-lock
claim).

Two layers, mirroring ``test_exec_bits.py``:

* synthetic ``tmp_path`` trees exercising every branch of the scan, and
* a real-tree assertion that the shipped ``cli/`` carries no duplicate — the
  known-positive/known-negative proof the plan's §3 requires, now durable rather
  than a one-off manual grep.
"""

from pathlib import Path

from tasks.lint import store_duplication

REPO_ROOT = Path(__file__).resolve().parents[3]


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_flags_a_temp_rename_outside_store(tmp_path: Path) -> None:
    _write(tmp_path, "cli/config-adapters/src/store.rs", "fs::rename(a, b)?;\n")
    assert store_duplication.violations(tmp_path) == [
        "cli/config-adapters/src/store.rs:1"
    ]


def test_flags_every_shape(tmp_path: Path) -> None:
    _write(tmp_path, "cli/a/src/x.rs", "let t = NamedTempFile::new()?;\n")
    _write(tmp_path, "cli/b/src/y.rs", "temp.persist(target)?;\n")
    flagged = store_duplication.violations(tmp_path)
    assert "cli/a/src/x.rs:1" in flagged
    assert "cli/b/src/y.rs:1" in flagged


def test_does_not_flag_the_store_crate(tmp_path: Path) -> None:
    _write(tmp_path, "cli/store/src/lib.rs", "fs::rename(a, b)?;\n")
    assert store_duplication.violations(tmp_path) == []


def test_does_not_flag_the_allowlisted_renames(tmp_path: Path) -> None:
    for rel in store_duplication.ALLOWLIST:
        _write(tmp_path, rel, "fs::rename(a, b)?;\n")
    assert store_duplication.violations(tmp_path) == []


def test_does_not_flag_non_src_files(tmp_path: Path) -> None:
    # A rename in a test file (not under /src/) is not a shipped primitive.
    _write(tmp_path, "cli/a/tests/x.rs", "fs::rename(a, b)?;\n")
    assert store_duplication.violations(tmp_path) == []


def test_the_real_cli_tree_carries_no_duplicate() -> None:
    # The guard proper: after consolidation cli/ has one atomic_write.
    assert store_duplication.violations(REPO_ROOT) == []
