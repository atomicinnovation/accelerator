from typing import Any
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks import deps
from tasks.shared.rust import PUP_NIGHTLY, PUP_VERSION

_PUP_COMPONENTS = ("rustc-dev", "rust-src", "llvm-tools-preview")
_PRESENT_VERSION = (
    f"\x1b[1mcargo-pup version\x1b[0m \x1b[32m{PUP_VERSION}\x1b[0m"
)
_ABSENT_VERSION = "cargo-pup version 0.0.0"


def _commands(ctx: MagicMock) -> list[str]:
    return [call.args[0] for call in ctx.run.call_args_list]


def _runner(*, version_stdout: str, fail: str | None = None):
    def run(command: str, **_kwargs: Any) -> MagicMock:
        exited = 1 if fail is not None and fail in command else 0
        stdout = version_stdout if "pup --version" in command else ""
        return MagicMock(exited=exited, stdout=stdout)

    return run


@pytest.fixture
def ctx() -> MagicMock:
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=0, stdout="")
    return m


class TestInstallRustComponents:
    def test_adds_rustfmt_clippy_and_llvm_tools(self, ctx: MagicMock):
        deps.install_rust_components(ctx)
        assert (
            "rustup component add rustfmt clippy llvm-tools-preview"
            in _commands(ctx)
        )


class TestInstallPup:
    def test_installs_the_pinned_nightly_with_all_components(
        self, ctx: MagicMock
    ):
        ctx.run.side_effect = _runner(version_stdout=_PRESENT_VERSION)
        deps.install_pup(ctx)
        rustup = next(
            c
            for c in _commands(ctx)
            if c.startswith("rustup toolchain install")
        )
        assert PUP_NIGHTLY in rustup
        for component in _PUP_COMPONENTS:
            assert f"--component {component}" in rustup

    def test_skips_install_when_pinned_version_already_present(
        self, ctx: MagicMock
    ):
        ctx.run.side_effect = _runner(version_stdout=_PRESENT_VERSION)
        deps.install_pup(ctx)
        assert not any("install cargo_pup" in c for c in _commands(ctx))

    def test_installs_pinned_cargo_pup_when_absent(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(version_stdout=_ABSENT_VERSION)
        deps.install_pup(ctx)
        assert (
            f"cargo +{PUP_NIGHTLY} install cargo_pup "
            f"--version {PUP_VERSION} --locked"
        ) in _commands(ctx)

    def test_runs_the_override_preflight(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(version_stdout=_PRESENT_VERSION)
        deps.install_pup(ctx)
        assert f"cargo +{PUP_NIGHTLY} --version" in _commands(ctx)

    def test_nightly_install_failure_reraised_as_exit_naming_pin(
        self, ctx: MagicMock
    ):
        # The actionable-error contract: a GC'd nightly yields an Exit naming
        # PUP_NIGHTLY, not a raw rustup stack trace. Mocked — no real nightly.
        ctx.run.side_effect = _runner(
            version_stdout=_PRESENT_VERSION, fail="rustup toolchain install"
        )
        with pytest.raises(Exit) as exc_info:
            deps.install_pup(ctx)
        assert PUP_NIGHTLY in str(exc_info.value)

    def test_raises_when_override_preflight_fails(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(
            version_stdout=_PRESENT_VERSION,
            fail=f"cargo +{PUP_NIGHTLY} --version",
        )
        with pytest.raises(Exit):
            deps.install_pup(ctx)
