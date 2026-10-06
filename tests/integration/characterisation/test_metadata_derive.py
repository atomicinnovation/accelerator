import pytest

from tests.integration.support.characterisation import (
    FILENAME_STAMP,
    NOW,
    REVISION,
)

_MASKS = [NOW, FILENAME_STAMP, REVISION]


def _committed(repository):
    repository.write("README.md", "fixture\n")
    repository.commit()
    return repository


@pytest.mark.parametrize(
    "timestamp_format", ["date-time-underscored", "compact-time", "date-only"]
)
def test_derive_in_a_clean_working_copy(
    repository, binaries, run, matches_golden, timestamp_format
):
    _committed(repository)

    matches_golden(
        run(
            binaries.subbinary("corpus"),
            "metadata",
            "derive",
            f"--filename-timestamp-format={timestamp_format}",
            cwd=repository.root,
        ),
        [*repository.revisions(), *_MASKS],
    )


def test_derive_in_a_dirty_working_copy(
    repository, binaries, run, matches_golden
):
    _committed(repository).write("README.md", "edited\n")

    matches_golden(
        run(
            binaries.subbinary("corpus"),
            "metadata",
            "derive",
            cwd=repository.root,
        ),
        [*repository.revisions(), *_MASKS],
    )


def test_derive_before_the_first_commit(
    repository, binaries, run, matches_golden
):
    matches_golden(
        run(
            binaries.subbinary("corpus"),
            "metadata",
            "derive",
            cwd=repository.root,
        ),
        [*repository.revisions(), *_MASKS],
    )


@pytest.mark.vcs_specific
def test_derive_outside_any_repository(
    fixture_root, binaries, run, matches_golden
):
    plain = fixture_root / "plain"
    plain.mkdir()

    matches_golden(
        run(binaries.subbinary("corpus"), "metadata", "derive", cwd=plain),
        _MASKS,
    )
