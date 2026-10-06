import shutil
from pathlib import Path

import pytest

CORPUS = Path(__file__).parent / "fixtures" / "frontmatter-corpus"

_ROOT_CLASSES = [
    "0018-absent.md",
    "0019-unclosed.md",
    "0020-sequence-root.md",
    "0021-scalar-root.md",
    "0022-null-root.md",
    "0023-empty-root.md",
    "0024-tagged.md",
]


@pytest.fixture
def corpus(repository):
    shutil.copytree(CORPUS, repository.root, dirs_exist_ok=True)
    repository.commit()
    return repository


def _validate(run, binaries, cwd, *args):
    return run(
        binaries.subbinary("corpus"), "frontmatter", "validate", *args, cwd=cwd
    )


@pytest.mark.parametrize(
    "checks", [None, "structure", "references"], ids=lambda c: c or "default"
)
def test_validate_walks_the_whole_corpus(
    corpus, binaries, run, matches_golden, checks
):
    args = ("--checks", checks) if checks else ()
    matches_golden(_validate(run, binaries, corpus.root, *args))


@pytest.mark.parametrize("name", _ROOT_CLASSES)
def test_validate_classifies_a_frontmatter_root(
    corpus, binaries, run, matches_golden, name
):
    matches_golden(
        _validate(run, binaries, corpus.root, "--file", f"meta/work/{name}")
    )


def test_validate_a_valid_file(corpus, binaries, run, matches_golden):
    matches_golden(
        _validate(
            run, binaries, corpus.root, "--file", "meta/work/0001-valid.md"
        )
    )
