from pathlib import Path

import pytest

from tasks.shared.instrumented_build import (
    Sweep,
    UnitListing,
    parse_build_environment,
    superseded_entries,
)

UNITS = (
    Path(__file__).parent / "fixtures/coverage/cargo-units.jsonl"
).read_text()

SMALLVEC = "0e524c8a43f44c11"
KERNEL_TESTS = "418d5fa629a38632"
PROC_MACRO2_BUILD_SCRIPT = "4b24466e4387f5e4"
PROC_MACRO2_BUILD_OUTPUT = "5ce08310c6c494bc"
SUPERSEDED = "3d4b3aa50bf88d80"


class TestUnitListing:
    def test_reads_the_hash_of_every_unit_in_a_real_listing(self):
        listing = UnitListing.parse(UNITS)

        assert listing.live_hashes == {
            SMALLVEC,
            KERNEL_TESTS,
            PROC_MACRO2_BUILD_SCRIPT,
            PROC_MACRO2_BUILD_OUTPUT,
        }

    def test_a_fresh_listing_rebuilt_nothing(self):
        assert UnitListing.parse(UNITS).rebuilt == 0

    def test_counts_each_unit_that_was_not_fresh(self):
        messages = UNITS.replace('"fresh":true', '"fresh":false', 2)

        assert UnitListing.parse(messages).rebuilt == 2

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
