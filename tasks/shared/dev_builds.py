"""Debug builds of the cli/ binaries the integration suites run."""

from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType

from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML, DISPATCHED_SUBBINARIES

# The server builds through build:server:dev, which needs its dev-frontend
# feature set rather than a plain debug build.
_SERVER = "visualiser"

_PACKAGES_NAMED_AFTER_THEIR_DIRECTORY = ("jira", "linear")

# The feature admits a loopback API URL. Every debug build of these binaries
# carries it, because two builds of one target/debug path with different
# features would overwrite each other's binary.
_LOOPBACK_SEAMED = ("jira", "linear", "research")


@dataclass(frozen=True)
class DevBinary:
    name: str
    package: str
    loopback_seam: bool = False

    @property
    def features(self) -> tuple[str, ...]:
        if self.loopback_seam:
            return (f"{self.package}/test-loopback",)
        return ()


def _subbinary(token: str) -> DevBinary:
    package = (
        f"{token}-cli"
        if token in _PACKAGES_NAMED_AFTER_THEIR_DIRECTORY
        else f"accelerator-{token}"
    )
    return DevBinary(
        f"accelerator-{token}",
        package=package,
        loopback_seam=token in _LOOPBACK_SEAMED,
    )


LAUNCHER = DevBinary("accelerator", package="accelerator")

_SUBBINARY_TOKENS = tuple(
    token for token in DISPATCHED_SUBBINARIES if token != _SERVER
)
SUBBINARIES = tuple(_subbinary(token) for token in _SUBBINARY_TOKENS)

DEV_BUILDS: Mapping[str, tuple[DevBinary, ...]] = MappingProxyType(
    {
        "launcher": (LAUNCHER,),
        **{
            token: (binary,)
            for token, binary in zip(
                _SUBBINARY_TOKENS, SUBBINARIES, strict=True
            )
        },
        "characterisation": (LAUNCHER, *SUBBINARIES),
    }
)


def cargo_build_command(
    binaries: Iterable[DevBinary],
    manifest: Path = CLI_WORKSPACE_CARGO_TOML,
) -> str:
    binaries = tuple(binaries)
    selection = " ".join(f"--bin {binary.name}" for binary in binaries)
    command = f"cargo build --manifest-path {manifest} {selection}"
    features = [f for binary in binaries for f in binary.features]
    if features:
        command += f" --features {','.join(features)}"
    return command
