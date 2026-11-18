import os
from pathlib import Path

import pytest

from tasks.shared.instrumented_build import (
    Sweep,
    UnitListing,
    UnmatchedUpliftError,
    live_units,
    parse_build_environment,
    superseded_entries,
    superseded_incremental,
)

UNITS = (
    Path(__file__).parent / "fixtures/coverage/cargo-units.jsonl"
).read_text()

SMALLVEC = "0e524c8a43f44c11"
KERNEL_TESTS = "418d5fa629a38632"
PROC_MACRO2_BUILD_SCRIPT = "4b24466e4387f5e4"
PROC_MACRO2_BUILD_OUTPUT = "5ce08310c6c494bc"
SUPERSEDED = "3d4b3aa50bf88d80"
UPLIFTED_VERIFY = Path(
    "/workspace/cli/target/llvm-cov-target/debug/accelerator-verify"
)


class TestUnitListing:
    def test_reads_the_hash_of_every_unit_in_a_real_listing(self):
        listing = UnitListing.parse(UNITS)

        assert listing.live_hashes == {
            SMALLVEC,
            KERNEL_TESTS,
            PROC_MACRO2_BUILD_SCRIPT,
            PROC_MACRO2_BUILD_OUTPUT,
        }

    def test_keeps_the_artefacts_cargo_reports_without_a_hash(self):
        assert UnitListing.parse(UNITS).uplifted == {UPLIFTED_VERIFY}

    def test_counts_the_units_of_each_crate(self):
        assert UnitListing.parse(UNITS).crates == {
            "smallvec": 1,
            "kernel": 1,
            "accelerator_verify": 1,
            "build_script_build": 1,
        }

    def test_a_fresh_listing_rebuilt_nothing(self):
        assert UnitListing.parse(UNITS).rebuilt == 0

    def test_counts_each_unit_that_was_not_fresh(self):
        messages = UNITS.replace('"fresh":true', '"fresh":false', 3)

        assert UnitListing.parse(messages).rebuilt == 3

    def test_a_listing_with_no_messages_has_no_live_units(self):
        assert UnitListing.parse("").live_hashes == frozenset()


def _entry(build_dir: Path, relative: str, size: int = 0) -> Path:
    path = build_dir / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * size)
    return path


def _directory(build_dir: Path, relative: str) -> Path:
    path = build_dir / relative
    path.mkdir(parents=True)
    return path


def _file(build_dir: Path, relative: str, content: bytes) -> Path:
    path = _entry(build_dir, relative)
    path.write_bytes(content)
    return path


class TestLiveUnits:
    @pytest.fixture
    def build_dir(self, tmp_path: Path) -> Path:
        return tmp_path / "debug"

    def _listing(self, *uplifted: Path) -> UnitListing:
        return UnitListing(frozenset({SMALLVEC}), frozenset(uplifted), {}, 0)

    def test_hashed_artefacts_are_live(self):
        assert live_units(self._listing()) == {SMALLVEC}

    def test_an_uplifted_binary_lives_through_its_identical_twin(
        self, build_dir: Path
    ):
        uplifted = _file(build_dir, "accelerator-verify", b"current")
        _file(build_dir, f"deps/accelerator_verify-{KERNEL_TESTS}", b"current")

        assert live_units(self._listing(uplifted)) == {SMALLVEC, KERNEL_TESTS}

    @pytest.mark.parametrize(
        ("crate", "content"),
        [
            ("accelerator_verify", b"older"),
            ("accelerator_verify", b"an older build"),
            ("accelerator_vcs", b"current"),
        ],
    )
    def test_only_an_identical_build_of_the_same_crate_is_its_twin(
        self, build_dir: Path, crate: str, content: bytes
    ):
        uplifted = _file(build_dir, "accelerator-verify", b"current")
        _file(build_dir, f"deps/accelerator_verify-{KERNEL_TESTS}", b"current")
        _file(build_dir, f"deps/{crate}-{SUPERSEDED}", content)

        assert SUPERSEDED not in live_units(self._listing(uplifted))

    def test_refuses_an_uplifted_binary_without_a_twin(self, build_dir: Path):
        uplifted = _file(build_dir, "accelerator-verify", b"current")
        _file(build_dir, f"deps/accelerator_verify-{SUPERSEDED}", b"older")

        with pytest.raises(UnmatchedUpliftError, match="accelerator-verify"):
            live_units(self._listing(uplifted))


class TestSupersededEntries:
    @pytest.fixture
    def build_dir(self, tmp_path: Path) -> Path:
        return tmp_path / "debug"

    def test_returns_entries_whose_unit_is_not_live(self, build_dir: Path):
        _entry(build_dir, f"deps/libsmallvec-{SMALLVEC}.rlib")
        stale = _entry(build_dir, f"deps/libsmallvec-{SUPERSEDED}.rlib")

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == [stale]

    @pytest.mark.parametrize("swept", ["deps", "build", ".fingerprint"])
    def test_sweeps_each_unit_directory(self, build_dir: Path, swept: str):
        stale = _directory(build_dir, f"{swept}/smallvec-{SUPERSEDED}")

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == [stale]

    def test_returns_files_and_directories(self, build_dir: Path):
        binary = _entry(build_dir, f"deps/kernel-{SUPERSEDED}")
        symbols = _directory(build_dir, f"deps/kernel-{SUPERSEDED}.dSYM")

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == [
            binary,
            symbols,
        ]

    @pytest.mark.parametrize(
        "name",
        [
            f"libsmallvec-{SUPERSEDED}.rlib",
            f"kernel-{SUPERSEDED}",
            f"kernel-{SUPERSEDED}.d",
        ],
    )
    def test_reads_the_hash_that_ends_the_unit_name(
        self, build_dir: Path, name: str
    ):
        stale = _entry(build_dir, f"deps/{name}")

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == [stale]

    def test_reads_the_last_hash_in_a_name(self, build_dir: Path):
        _entry(build_dir, f"deps/tool-{SUPERSEDED}-{SMALLVEC}")

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == []

    @pytest.mark.parametrize(
        "kept",
        [
            "deps/libunhashed.rlib",
            f"incremental/kernel-{SUPERSEDED}",
            f"kernel-{SUPERSEDED}",
        ],
    )
    def test_keeps_entries_outside_the_swept_unit_directories(
        self, build_dir: Path, kept: str
    ):
        _entry(build_dir, kept)

        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == []

    def test_a_build_dir_without_unit_directories_has_nothing_superseded(
        self, build_dir: Path
    ):
        assert superseded_entries(build_dir, frozenset({SMALLVEC})) == []

    def test_refuses_to_judge_without_live_units(self, build_dir: Path):
        _entry(build_dir, f"deps/libsmallvec-{SMALLVEC}.rlib")

        with pytest.raises(ValueError, match="no live units"):
            superseded_entries(build_dir, frozenset())


def _session(build_dir: Path, name: str, modified: int) -> Path:
    path = _directory(build_dir, f"incremental/{name}")
    os.utime(path, (modified, modified))
    return path


class TestSupersededIncremental:
    @pytest.fixture
    def build_dir(self, tmp_path: Path) -> Path:
        return tmp_path / "debug"

    def test_keeps_the_newest_directory_for_each_unit_of_a_crate(
        self, build_dir: Path
    ):
        stale = _session(build_dir, "kernel-0older", 100)
        _session(build_dir, "kernel-1newer", 300)
        _session(build_dir, "kernel-2newest", 200)

        assert superseded_incremental(build_dir, {"kernel": 2}) == [stale]

    def test_sweeps_every_directory_of_a_crate_with_no_units(
        self, build_dir: Path
    ):
        _session(build_dir, "kernel-1newer", 300)
        stale = _session(build_dir, "retired-0older", 100)

        assert superseded_incremental(build_dir, {"kernel": 1}) == [stale]

    def test_reads_the_crate_name_before_the_last_hyphen(self, build_dir: Path):
        stale = _session(build_dir, "build_script_build-0older", 100)
        _session(build_dir, "build_script_build-1newer", 300)

        assert superseded_incremental(build_dir, {"build_script_build": 1}) == [
            stale
        ]

    def test_a_build_dir_without_incremental_has_nothing_superseded(
        self, build_dir: Path
    ):
        assert superseded_incremental(build_dir, {"kernel": 1}) == []

    def test_refuses_to_judge_without_units(self, build_dir: Path):
        _session(build_dir, "kernel-1newer", 300)

        with pytest.raises(ValueError, match="no live units"):
            superseded_incremental(build_dir, {})


class TestSweep:
    @pytest.mark.parametrize(
        ("freed", "shown"),
        [
            (0, "0 B"),
            (999, "999 B"),
            (1_500, "1.50 kB"),
            (2_280_000_000, "2.28 GB"),
        ],
    )
    def test_summarises_the_entries_and_the_space_freed(
        self, freed: int, shown: str
    ):
        assert Sweep(entries=3, freed_bytes=freed).summary_line() == (
            f"swept 3 superseded artefacts ({shown})"
        )


class TestParseBuildEnvironment:
    def test_unquotes_single_quoted_values(self):
        environment = parse_build_environment(
            "LLVM_PROFILE_FILE='/workspace/cli/target/cli-%p-%16m.profraw'\n"
        )

        assert environment == {
            "LLVM_PROFILE_FILE": "/workspace/cli/target/cli-%p-%16m.profraw"
        }

    def test_keeps_unquoted_values_as_they_are(self):
        environment = parse_build_environment(
            "CARGO_LLVM_COV=1\nRUSTC_WRAPPER=/opt/bin/cargo-llvm-cov\n"
        )

        assert environment == {
            "CARGO_LLVM_COV": "1",
            "RUSTC_WRAPPER": "/opt/bin/cargo-llvm-cov",
        }

    def test_ignores_lines_that_set_nothing(self):
        assert parse_build_environment("\ninfo: setting cfg(coverage)\n") == {}
