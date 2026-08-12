from typing import Any
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks import deps
from tasks.shared.rust import PUBLIC_API_VERSION, PUP_VERSION, RUST_NIGHTLY

_NIGHTLY_COMPONENTS = ("rustc-dev", "rust-src", "llvm-tools-preview")
_PRESENT_VERSION = (
    f"\x1b[1mcargo-pup version\x1b[0m \x1b[32m{PUP_VERSION}\x1b[0m"
)
_ABSENT_VERSION = "cargo-pup version 0.0.0"

_PUBLIC_API_PRESENT_VERSION = f"cargo-public-api {PUBLIC_API_VERSION}"
_PUBLIC_API_ABSENT_VERSION = "cargo-public-api 0.0.0"


def _commands(ctx: MagicMock) -> list[str]:
    return [call.args[0] for call in ctx.run.call_args_list]


def _runner(
    *,
    version_stdout: str = "",
    public_api_version_stdout: str = "",
    fail: str | None = None,
):
    def run(command: str, **_kwargs: Any) -> MagicMock:
        exited = 1 if fail is not None and fail in command else 0
        if "pup --version" in command:
            stdout = version_stdout
        elif command == "cargo public-api --version":
            stdout = public_api_version_stdout
        else:
            stdout = ""
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


class TestInstallNightly:
    def test_installs_the_pinned_nightly_with_all_components(
        self, ctx: MagicMock
    ):
        deps.install_nightly(ctx)
        rustup = next(
            c
            for c in _commands(ctx)
            if c.startswith("rustup toolchain install")
        )
        assert RUST_NIGHTLY in rustup
        for component in _NIGHTLY_COMPONENTS:
            assert f"--component {component}" in rustup

    def test_runs_the_override_preflight(self, ctx: MagicMock):
        deps.install_nightly(ctx)
        assert f"cargo +{RUST_NIGHTLY} --version" in _commands(ctx)

    def test_installs_no_nightly_lane_tool(self, ctx: MagicMock):
        # The toolchain task provisions the toolchain and nothing else: each
        # tool that reaches for it owns its own install task.
        deps.install_nightly(ctx)
        assert not any("cargo install" in c for c in _commands(ctx))

    def test_install_failure_reraised_as_exit_naming_pin(self, ctx: MagicMock):
        # The actionable-error contract: a GC'd nightly yields an Exit naming
        # RUST_NIGHTLY, not a raw rustup stack trace. Mocked — no real nightly.
        ctx.run.side_effect = _runner(fail="rustup toolchain install")
        with pytest.raises(Exit) as exc_info:
            deps.install_nightly(ctx)
        assert RUST_NIGHTLY in str(exc_info.value)

    def test_raises_when_override_preflight_fails(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(fail=f"cargo +{RUST_NIGHTLY} --version")
        with pytest.raises(Exit):
            deps.install_nightly(ctx)


class TestInstallPup:
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
            f"cargo +{RUST_NIGHTLY} install cargo_pup "
            f"--version {PUP_VERSION} --locked"
        ) in _commands(ctx)

    def test_provisions_no_toolchain(self, ctx: MagicMock):
        # The nightly arrives via the deps:install:nightly edge, so a second
        # rustup install here would race the first on ~/.rustup.
        ctx.run.side_effect = _runner(version_stdout=_ABSENT_VERSION)
        deps.install_pup(ctx)
        assert not any("rustup toolchain install" in c for c in _commands(ctx))

    def test_install_failure_reraised_as_exit_naming_pin(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(
            version_stdout=_ABSENT_VERSION, fail="install cargo_pup"
        )
        with pytest.raises(Exit) as exc_info:
            deps.install_pup(ctx)
        assert PUP_VERSION in str(exc_info.value)


class TestInstallPublicApi:
    def test_skips_install_when_pinned_version_already_present(
        self, ctx: MagicMock
    ):
        ctx.run.side_effect = _runner(
            public_api_version_stdout=_PUBLIC_API_PRESENT_VERSION
        )
        deps.install_public_api(ctx)
        assert not any("install cargo-public-api" in c for c in _commands(ctx))

    def test_installs_pinned_cargo_public_api_when_absent(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(
            public_api_version_stdout=_PUBLIC_API_ABSENT_VERSION
        )
        deps.install_public_api(ctx)
        assert (
            f"cargo install cargo-public-api --version {PUBLIC_API_VERSION} "
            "--locked"
        ) in _commands(ctx)

    def test_installs_on_stable_and_provisions_no_toolchain(
        self, ctx: MagicMock
    ):
        # cargo-public-api has no rustc_private driver, so it builds on stable;
        # the nightly it shells out to at check time arrives via the
        # deps:install:nightly edge, never from here.
        ctx.run.side_effect = _runner(
            public_api_version_stdout=_PUBLIC_API_ABSENT_VERSION
        )
        deps.install_public_api(ctx)
        commands = _commands(ctx)
        assert not any("rustup toolchain install" in c for c in commands)
        assert not any(f"+{RUST_NIGHTLY}" in c for c in commands)

    def test_install_failure_reraised_as_exit_naming_pin(self, ctx: MagicMock):
        ctx.run.side_effect = _runner(
            public_api_version_stdout=_PUBLIC_API_ABSENT_VERSION,
            fail="cargo install cargo-public-api",
        )
        with pytest.raises(Exit) as exc_info:
            deps.install_public_api(ctx)
        assert PUBLIC_API_VERSION in str(exc_info.value)

    def test_version_probe_does_not_substring_match(self, ctx: MagicMock):
        # PUBLIC_API_VERSION must be a whole token: a probe reporting a
        # version carrying the pin as a strict prefix (e.g. "0.52.01" against
        # a "0.52.0" pin) must not be read as present.
        ctx.run.side_effect = _runner(
            public_api_version_stdout=(
                f"cargo-public-api {PUBLIC_API_VERSION}1"
            )
        )
        deps.install_public_api(ctx)
        assert any("install cargo-public-api" in c for c in _commands(ctx))
