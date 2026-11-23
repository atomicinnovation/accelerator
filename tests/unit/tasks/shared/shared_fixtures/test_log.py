import json
from pathlib import Path

import pytest

from tasks.shared.build_measurement import timeline
from tasks.shared.hashing import TreeChanges, TreeDigest
from tasks.shared.paths import REPO_ROOT
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    PRIVATE_SETS,
    WORKBENCHES_VARIABLE,
    Family,
)
from tasks.shared.shared_fixtures.log import (
    Adoption,
    Copy,
    DigestPhase,
    FinishedRun,
    FixtureLog,
    FixtureLogs,
    PrivateBuild,
    SharedBuild,
    TreeChangesRecorded,
    TreeDigestRecorded,
    TreeDigests,
)

CONTRACT = REPO_ROOT / "cli/vcs-test-support/contract"
LOG_FORMAT = CONTRACT / "fixture-log.jsonl"
VARIABLES = CONTRACT / "variables.json"
FLAGS = CONTRACT / "flags.json"

TREE = Path("/tmp/accelerator-shared-fixtures-501/run/matrix")
CLASSIFY = "vcs-adapters::classify"
GOLDENS = "accelerator-vcs::status_log_goldens"
MATRIX_CONSUMERS = (
    "vcs-adapters::classify",
    "vcs-adapters::queries",
    "vcs-adapters::scrub",
    "vcs-test-support::matrix",
    "vcs-test-support::matrix_private_build",
)
WORKBENCH_C = Path(
    "/tmp/accelerator-shared-fixtures-501/run/workbenches/vcs-workbench-c"
)
RUN_ID = "20261119T101500123456Z-0b6f9a0e-6c1d-4f0a-9d7e-2c1b3a4d5e6f"


def _run_id(n: int) -> str:
    return f"20261119T1015{n:08d}Z-0b6f9a0e-6c1d-4f0a-9d7e-2c1b3a4d5e6f"


def _shared_build(tree: Path = TREE) -> dict:
    return {
        "event": "shared-build",
        "family": "matrix",
        "tree": str(tree),
        "git": "git version 2.50.1",
        "jj": "jj 0.43.0",
    }


def _adoption(binary: str = CLASSIFY, test: str = "a_test") -> dict:
    return {
        "event": "adoption",
        "family": "matrix",
        "binary": binary,
        "test": test,
        "hermetic-root": f"/workbenches/{binary}-{test}",
    }


def _private_build(
    fixtures: str = "matrix", binary: str = CLASSIFY, test: str = "a_test"
) -> dict:
    return {
        "event": "private-build",
        "fixtures": fixtures,
        "binary": binary,
        "test": test,
        "hermetic-root": f"/tmp/vcs-workbench-{test}",
    }


def _copy(
    binary: str = GOLDENS,
    test: str = "a_test",
    named_by: tuple[str, ...] = (),
    family: str = "status-log",
) -> dict:
    workbench = f"/workbenches/{binary}-{test}"
    return {
        "event": "copy",
        "family": family,
        "binary": binary,
        "test": test,
        "hermetic-root": workbench,
        "copy": workbench,
        "named-by": list(named_by),
    }


def _a_clean_run(*extra: dict) -> list[dict]:
    return [
        _shared_build(),
        *(_adoption(binary) for binary in MATRIX_CONSUMERS),
        *extra,
    ]


def _log(tmp_path: Path, events: list[dict]) -> FixtureLog:
    path = tmp_path / "run.jsonl"
    path.write_text("".join(json.dumps(event) + "\n" for event in events))
    return FixtureLog(path)


def _digest(*paths: str, content: str = "x") -> TreeDigest:
    return TreeDigest(dict.fromkeys(paths, f"file:644:{content}"))


def _finished(
    *,
    digests: TreeDigests | None = None,
    filtered: bool = False,
    tests_passed: bool = True,
    trees: dict[Family, Path] | None = None,
) -> FinishedRun:
    unchanged = _digest("PG/.git/index")
    return FinishedRun(
        trees={Family.MATRIX: TREE} if trees is None else trees,
        digests={Family.MATRIX: digests or TreeDigests(unchanged, unchanged)},
        filtered=filtered,
        tests_passed=tests_passed,
    )


class TestTheSharedFormat:
    def test_every_kind_in_the_shared_format_is_read(self):
        events = FixtureLog(LOG_FORMAT).read().events

        assert events == (
            SharedBuild(Family.MATRIX, TREE, "git version 2.50.1", "jj 0.43.0"),
            Adoption(
                Family.MATRIX,
                "vcs-adapters::classify",
                "every_fixture_classifies_as_expected",
                Path(
                    "/tmp/accelerator-shared-fixtures-501/run/workbenches/"
                    "vcs-workbench-a"
                ),
            ),
            PrivateBuild(
                Family.MATRIX,
                "vcs-adapters::classify",
                "every_fixture_classifies_as_expected",
                Path("/tmp/vcs-workbench-b"),
            ),
            Copy(
                Family.STATUS_LOG,
                GOLDENS,
                "status_and_log_match_every_captured_state",
                WORKBENCH_C,
                WORKBENCH_C,
                (WORKBENCH_C / "conflict-git/.git/config",),
            ),
            TreeDigestRecorded(
                Family.MATRIX,
                TREE,
                DigestPhase.BUILT,
                TreeDigest({}).value,
            ),
            TreeChangesRecorded(
                Family.MATRIX,
                TREE,
                TreeChanges(
                    added=("poison.gitconfig",),
                    removed=(),
                    modified=("PG/.git/index",),
                ),
            ),
        )

    def test_a_private_build_of_the_private_sets_names_them(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, [_private_build("private-sets")])

        assert log.read().events == (
            PrivateBuild(
                PRIVATE_SETS,
                CLASSIFY,
                "a_test",
                Path("/tmp/vcs-workbench-a_test"),
            ),
        )

    def test_a_test_is_named_in_full_by_its_binary_and_its_name(self):
        adoption = Adoption(Family.MATRIX, CLASSIFY, "a_test", Path("/w"))

        assert adoption.full_name == f"{CLASSIFY} a_test"

    def test_an_event_without_its_binary_does_not_parse(self, tmp_path: Path):
        adoption = _adoption()
        del adoption["binary"]

        assert _log(tmp_path, [adoption]).read().unparsed == (1,)

    def test_a_missing_log_reads_as_empty(self, tmp_path: Path):
        contents = FixtureLog(tmp_path / "absent.jsonl").read()

        assert contents.events == ()
        assert contents.unparsed == ()

    def test_a_line_that_does_not_parse_is_kept_by_number(self, tmp_path: Path):
        path = tmp_path / "run.jsonl"
        path.write_text(
            json.dumps(_shared_build())
            + "\n{not json\n"
            + json.dumps({"event": "unheard-of"})
            + "\n"
        )

        assert FixtureLog(path).read().unparsed == (2, 3)

    @pytest.mark.parametrize("separator", ["\u0085", "\u2028", "\u2029"])
    def test_a_line_holding_a_unicode_line_separator_is_one_event(
        self, tmp_path: Path, separator: str
    ):
        path = tmp_path / "run.jsonl"
        tree = Path(f"/tmp/a{separator}b/matrix")
        path.write_text(
            json.dumps(_shared_build(tree), ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        contents = FixtureLog(path).read()

        assert contents.unparsed == ()
        assert [event.tree for event in contents.events] == [tree]

    def test_a_digest_is_recorded_in_the_shared_format(self, tmp_path: Path):
        log = FixtureLog(tmp_path / "run.jsonl")

        log.record_digest(
            Family.MATRIX, TREE, DigestPhase.BUILT, TreeDigest({})
        )

        assert log.path.read_text() == _contract_line("tree-digest")

    def test_changes_are_recorded_in_the_shared_format(self, tmp_path: Path):
        log = FixtureLog(tmp_path / "run.jsonl")

        log.record_changes(
            Family.MATRIX,
            TREE,
            TreeChanges(
                added=("poison.gitconfig",),
                removed=(),
                modified=("PG/.git/index",),
            ),
        )

        assert log.path.read_text() == _contract_line("tree-changes")

    def test_every_variable_matches_the_shared_names(self):
        names = json.loads(VARIABLES.read_text())

        assert names["handover"] == {
            family.value: family.handover_variable for family in Family
        }
        assert names["workbenches"] == WORKBENCHES_VARIABLE
        assert names["log"] == FIXTURE_LOG_VARIABLE

    def test_every_builder_flag_matches_the_shared_flags(self):
        flags = json.loads(FLAGS.read_text())

        assert flags == {family.value: family.flag for family in Family}


def _contract_line(kind: str) -> str:
    [line] = [
        line
        for line in LOG_FORMAT.read_text().splitlines(keepends=True)
        if f'"event":"{kind}"' in line
    ]
    return line


class TestTheSummary:
    def test_counts_what_each_family_was_used_by(self, tmp_path: Path):
        log = _log(
            tmp_path,
            [_shared_build(), *[_adoption(test=f"t{n}") for n in range(19)]],
        )

        assert log.summary(Path("/root")).lines() == [
            f"Shared VCS fixtures (log: {log.path}; shared root: /root)",
            "  matrix: 1 shared build, 19 adoptions, 0 copies, "
            "0 private builds",
            "  status-log set: 0 shared builds, 0 adoptions, 0 copies, "
            "0 private builds",
            "  private sets (never shared): 0 private builds",
        ]

    def test_counts_private_builds_of_each_kind(self, tmp_path: Path):
        log = _log(
            tmp_path,
            [
                _private_build("matrix"),
                _private_build("private-sets", test="a"),
                _private_build("private-sets", test="b"),
            ],
        )

        assert log.summary(Path("/root")).lines()[1:] == [
            "  matrix: 0 shared builds, 0 adoptions, 0 copies, 1 private build",
            "  status-log set: 0 shared builds, 0 adoptions, 0 copies, "
            "0 private builds",
            "  private sets (never shared): 2 private builds",
        ]

    def test_counts_the_copies_of_each_family(self, tmp_path: Path):
        log = _log(tmp_path, [_copy(test="a"), _copy(test="b")])

        assert log.summary(Path("/root")).lines()[2] == (
            "  status-log set: 0 shared builds, 0 adoptions, 2 copies, "
            "0 private builds"
        )

    def test_no_line_reads_as_a_nextest_summary_or_a_task_duration(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, _a_clean_run())

        for line in log.summary(Path("/root")).lines():
            assert not timeline.NEXTEST_SUMMARY.match(line)
            assert not timeline._FINISHED_IN.fullmatch(line)


class TestProblems:
    def test_a_clean_run_has_none(self, tmp_path: Path):
        assert _log(tmp_path, _a_clean_run()).problems(_finished()) == []

    def test_a_changed_tree_names_its_changes_and_how_to_find_the_writer(
        self, tmp_path: Path
    ):
        built = _digest("PG/.git/index")
        removed = TreeDigest(
            {
                **_digest("PG/.git/index", content="y").entries,
                "poison.gitconfig": "f",
            }
        )

        [problem] = _log(tmp_path, _a_clean_run()).problems(
            _finished(digests=TreeDigests(built, removed))
        )

        assert problem.startswith(
            f"the matrix tree {TREE} changed during the run: added "
            "poison.gitconfig; modified PG/.git/index"
        )
        assert "the tree has been removed" in problem
        for consumer in MATRIX_CONSUMERS:
            assert (
                f"mise run test:unit:cli --filter 'binary_id({consumer})'"
                in problem
            )

    def test_a_long_list_of_changes_is_cut_short(self, tmp_path: Path):
        built = _digest()
        removed = _digest(*(f"file{n:02d}" for n in range(13)))

        [problem] = _log(tmp_path, _a_clean_run()).problems(
            _finished(digests=TreeDigests(built, removed))
        )

        assert "file09" in problem
        assert "file10" not in problem
        assert "and 3 more" in problem

    def test_a_private_build_of_a_handed_over_family_names_the_bypass(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, _a_clean_run(_private_build(test="sneaky")))

        assert log.problems(_finished()) == [
            f"{CLASSIFY} sneaky built the matrix privately although the run "
            "handed it over through ACCELERATOR_SHARED_MATRIX; its Handover "
            "did not come from the environment; open it with "
            "Workbench::for_this_test(), or give a deliberately private "
            "Handover a test-local FixtureLog"
        ]

    def test_a_private_build_of_a_family_a_filtered_run_skipped_names_the_list(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, [_private_build(test="unlisted")])

        assert log.problems(_finished(trees={}, filtered=True)) == [
            f"{CLASSIFY} unlisted built the matrix privately; add {CLASSIFY} "
            "to the matrix consumers in "
            "tasks/shared/shared_fixtures/consumers.py"
        ]

    def test_a_private_build_of_the_private_sets_is_no_problem(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, _a_clean_run(_private_build("private-sets")))

        assert log.problems(_finished()) == []

    def test_a_line_that_does_not_parse_is_named_by_number(
        self, tmp_path: Path
    ):
        clean = _a_clean_run()
        log = _log(tmp_path, clean)
        with log.path.open("a") as file:
            file.write("{torn\n")
        torn = len(clean) + 1

        assert log.problems(_finished()) == [
            f"line {torn} of the fixture log {log.path} does not parse"
        ]

    @pytest.mark.parametrize(
        "events",
        [
            [],
            [_shared_build(Path("/elsewhere/matrix"))],
        ],
        ids=["no shared build", "a shared build of another tree"],
    )
    def test_a_built_family_needs_a_shared_build_of_its_tree(
        self, tmp_path: Path, events: list[dict]
    ):
        log = _log(
            tmp_path, [*events, *(_adoption(b) for b in MATRIX_CONSUMERS)]
        )

        assert log.problems(_finished()) == [
            f"the matrix was built but the log records no shared build of "
            f"{TREE}; the hand-over did not reach the builder"
        ]

    def test_a_missing_log_lacks_the_shared_build(self, tmp_path: Path):
        log = FixtureLog(tmp_path / "absent.jsonl")

        assert log.problems(_finished(tests_passed=False)) == [
            f"the matrix was built but the log records no shared build of "
            f"{TREE}; the hand-over did not reach the builder"
        ]

    def test_an_unlisted_consumer_of_an_unfiltered_run_is_named(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, _a_clean_run(_adoption("vcs-cli::status")))

        assert log.problems(_finished()) == [
            "vcs-cli::status uses the matrix but is not among its consumers "
            "in tasks/shared/shared_fixtures/consumers.py; add it there"
        ]

    def test_an_unlisted_consumer_of_a_filtered_run_is_not_judged(
        self, tmp_path: Path
    ):
        log = _log(tmp_path, _a_clean_run(_adoption("vcs-cli::status")))

        assert log.problems(_finished(filtered=True)) == []

    def test_a_silent_consumer_of_a_passing_unfiltered_run_is_named(
        self, tmp_path: Path
    ):
        log = _log(
            tmp_path,
            [_shared_build(), *(_adoption(b) for b in MATRIX_CONSUMERS[1:])],
        )

        assert log.problems(_finished()) == [
            f"{CLASSIFY} is listed as a matrix consumer but used no matrix; "
            "remove it from tasks/shared/shared_fixtures/consumers.py"
        ]

    @pytest.mark.parametrize(
        ("filtered", "tests_passed"),
        [(True, True), (False, False)],
        ids=["filtered", "failed"],
    )
    def test_a_silent_consumer_is_not_judged_unless_every_test_ran_and_passed(
        self, tmp_path: Path, filtered: bool, tests_passed: bool
    ):
        log = _log(tmp_path, [_shared_build()])

        assert (
            log.problems(
                _finished(filtered=filtered, tests_passed=tests_passed)
            )
            == []
        )

    def test_a_copy_still_naming_its_shared_tree_names_those_files(
        self, tmp_path: Path
    ):
        named = ("/w/conflict-git/.git/config", "/w/seed/.git/config")
        log = _log(tmp_path, _a_clean_run(_copy(named_by=named)))

        assert log.problems(_finished(filtered=True)) == [
            f"{GOLDENS} a_test copied the status-log set into "
            f"/workbenches/{GOLDENS}-a_test, but these copied files still "
            "name the shared tree, so the test would read or write it "
            "through them: /w/conflict-git/.git/config, /w/seed/.git/config"
        ]

    def test_a_clean_copy_is_no_problem(self, tmp_path: Path):
        log = _log(tmp_path, _a_clean_run(_copy()))

        assert log.problems(_finished(filtered=True)) == []

    def test_an_unlisted_consumer_that_copies_is_named(self, tmp_path: Path):
        log = _log(tmp_path, _a_clean_run(_copy("vcs-cli::status")))

        assert log.problems(_finished()) == [
            "vcs-cli::status uses the status-log set but is not among its "
            "consumers in tasks/shared/shared_fixtures/consumers.py; add it "
            "there"
        ]

    def test_a_consumer_that_only_copies_is_not_silent(self, tmp_path: Path):
        log = _log(
            tmp_path,
            [
                _shared_build(),
                _copy(CLASSIFY, family="matrix"),
                *(_adoption(b) for b in MATRIX_CONSUMERS[1:]),
            ],
        )

        assert log.problems(_finished()) == []

    def test_a_tree_without_both_digests_is_not_judged_changed(
        self, tmp_path: Path
    ):
        run = FinishedRun(
            trees={Family.MATRIX: TREE},
            digests={},
            filtered=False,
            tests_passed=True,
        )

        assert _log(tmp_path, _a_clean_run()).problems(run) == []


class TestFixtureLogs:
    def test_creating_a_log_creates_that_file_alone(self, tmp_path: Path):
        logs = tmp_path / "vcs-fixture-logs"

        log = FixtureLogs(logs).create(RUN_ID)

        assert log.path == logs / f"{RUN_ID}.jsonl"
        assert list(logs.iterdir()) == [log.path]
        assert log.path.read_text() == ""

    def test_pruning_keeps_the_newest_logs(self, tmp_path: Path):
        logs = FixtureLogs(tmp_path)
        for n in range(5):
            logs.create(_run_id(n))

        logs.prune(keep=2, live=frozenset())

        assert sorted(path.name for path in tmp_path.iterdir()) == [
            f"{_run_id(3)}.jsonl",
            f"{_run_id(4)}.jsonl",
        ]

    def test_pruning_never_removes_a_live_run_s_log(self, tmp_path: Path):
        logs = FixtureLogs(tmp_path)
        for n in range(3):
            logs.create(_run_id(n))

        logs.prune(keep=1, live=frozenset({_run_id(0)}))

        assert sorted(path.name for path in tmp_path.iterdir()) == [
            f"{_run_id(0)}.jsonl",
            f"{_run_id(2)}.jsonl",
        ]

    def test_pruning_leaves_files_that_are_not_run_logs(self, tmp_path: Path):
        logs = FixtureLogs(tmp_path)
        (tmp_path / "notes.jsonl").write_text("mine")
        logs.create(_run_id(0))

        logs.prune(keep=0, live=frozenset())

        assert [path.name for path in tmp_path.iterdir()] == ["notes.jsonl"]

    def test_pruning_a_log_another_run_already_pruned_succeeds(
        self, tmp_path: Path
    ):
        class PrunedConcurrently(FixtureLogs):
            def listed(self) -> list[Path]:
                listing = super().listed()
                listing[0].unlink()
                return listing

        logs = PrunedConcurrently(tmp_path)
        for n in range(2):
            logs.create(_run_id(n))

        logs.prune(keep=0, live=frozenset())

        assert list(tmp_path.iterdir()) == []

    def test_pruning_a_missing_directory_does_nothing(self, tmp_path: Path):
        FixtureLogs(tmp_path / "absent").prune(keep=20, live=frozenset())
