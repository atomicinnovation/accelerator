from pathlib import Path

from tasks.lint import corpus_identity

REPO_ROOT = Path(__file__).resolve().parents[3]


def _write(root: Path, rel: str, body: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body)


def test_a_corpus_file_naming_the_scheme_is_flagged(tmp_path: Path) -> None:
    _write(tmp_path, "cli/corpus/src/slug.rs", "use WorkItemIdScheme;\n")
    _write(tmp_path, "cli/corpus-adapters/src/scan.rs", "RegexScanner::new()\n")
    assert corpus_identity.violations(tmp_path) == [
        "cli/corpus-adapters/src/scan.rs",
        "cli/corpus/src/slug.rs",
    ]


def test_a_work_file_naming_the_scheme_is_not_flagged(tmp_path: Path) -> None:
    _write(
        tmp_path, "cli/work/src/id_scheme.rs", "pub struct WorkItemIdScheme;\n"
    )
    assert corpus_identity.violations(tmp_path) == []


def test_a_name_inside_a_longer_identifier_is_not_flagged(
    tmp_path: Path,
) -> None:
    _write(tmp_path, "cli/corpus/src/scan.rs", "struct IdScanned;\n")
    assert corpus_identity.violations(tmp_path) == []


def test_the_corpus_crates_hold_no_work_item_identity() -> None:
    assert corpus_identity.violations(REPO_ROOT) == []
