from tests.integration.support.characterisation import Repository


def test_the_retired_tracking_subcommand_is_unknown(
    repository: Repository, binaries, run, matches_golden
):
    result = run(
        binaries.launcher,
        "vcs",
        "tracking",
        "--path",
        "x",
        cwd=repository.root,
        env={"ACCELERATOR_VCS_BIN": str(binaries.subbinary("vcs"))},
    )

    assert result.code != 0
    assert "unrecognized subcommand 'tracking'" in result.stderr
    matches_golden(result)
