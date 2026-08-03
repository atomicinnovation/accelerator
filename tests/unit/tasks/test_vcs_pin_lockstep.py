"""The jj CLI and the jj-lib crate are a lockstep pair, and both VCS pins carry
their inline rationale.

The CLI writes the repository format the library reads, so a skew between
mise.toml's `jj` pin and cli/Cargo.toml's `jj-lib` pin fails in a way that reads
as an adapter defect rather than a pin mismatch — this repo has already hit
exactly that, with a local jj 0.42.0 against a jj-lib 0.43 design.

This checks *declarations in files*. That the binary actually building fixtures
matches the pin is asserted by the fixture harness, not here.

The comment assertions are not style policing: the work item requires both, and
comments are the first thing lost when a contributor regenerates a manifest on a
merge conflict — which the plan's lock-contention notes anticipate happening.
"""

import re
import tomllib
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_MISE = _REPO_ROOT / "mise.toml"
_CLI_CARGO = _REPO_ROOT / "cli/Cargo.toml"
_DENY = _REPO_ROOT / "cli/deny.toml"


def _minor(version: str) -> tuple[int, int]:
    parts = version.lstrip("=~^").split(".")
    return int(parts[0]), int(parts[1])


def _mise_jj() -> str:
    jj = tomllib.loads(_MISE.read_text())["tools"]["jj"]
    return jj["version"] if isinstance(jj, dict) else jj


def _workspace_dependency(name: str) -> str | dict:
    data = tomllib.loads(_CLI_CARGO.read_text())
    return data["workspace"]["dependencies"][name]


def _requirement(name: str) -> str:
    declared = _workspace_dependency(name)
    return declared if isinstance(declared, str) else declared["version"]


def _comment_above(text: str, needle: str) -> str:
    """The contiguous run of `#` lines immediately above the line matching."""
    lines = text.splitlines()
    index = next(
        position
        for position, line in enumerate(lines)
        if line.strip().startswith(needle)
    )
    comment = []
    cursor = index - 1
    while cursor >= 0 and lines[cursor].lstrip().startswith("#"):
        comment.append(lines[cursor].lstrip().lstrip("#").strip())
        cursor -= 1
    return "\n".join(reversed(comment)).strip()


def test_the_jj_cli_pin_and_the_jj_lib_pin_share_a_minor_version() -> None:
    cli = _mise_jj()
    crate = _requirement("jj-lib")
    assert _minor(cli) == _minor(crate), (
        f"jj CLI pin {cli} and jj-lib pin {crate} have drifted apart — the CLI "
        "writes the format the library reads, so bump both together"
    )


def test_the_jj_lib_pin_is_exact() -> None:
    # The crate declares its API unstable and the adapter leans on its
    # workspace-loader internals, so adopting a version is a deliberate act
    # rather than a resolution outcome.
    assert _requirement("jj-lib").startswith("="), (
        "jj-lib must stay exactly pinned"
    )


def test_the_gix_pin_permits_only_its_patch_line() -> None:
    # A tilde so a RustSec fix is a lock update rather than a pin edit, and
    # never a caret, which on a 0.x crate would still not cross 0.86 but says
    # something weaker than intended.
    requirement = _requirement("gix")
    assert requirement.startswith("~"), (
        f"gix should be tilde-pinned, found {requirement!r}"
    )


def test_the_mise_jj_pin_keeps_its_lockstep_comment() -> None:
    comment = _comment_above(_MISE.read_text(), "jj = ")
    assert comment, "the mise.toml jj pin has lost its comment"
    assert re.search(r"jj-lib", comment), (
        f"the mise.toml jj pin's comment no longer ties it to jj-lib: {comment}"
    )


def test_the_vcs_pins_keep_their_shared_comment() -> None:
    # The two pins are a matched pair introduced by one block, so the rationale
    # sits above the first of them rather than above each.
    comment = _comment_above(_CLI_CARGO.read_text(), "jj-lib = ")
    assert comment, "the cli/Cargo.toml VCS pins have lost their comment"
    for required in ("gix", "jj-lib"):
        assert required in comment, (
            f"the VCS pin comment no longer mentions {required}: {comment}"
        )


def test_the_gix_pin_comment_records_why_defaults_are_off() -> None:
    # default-features = false is the non-obvious half of the pin: it is what
    # keeps gix-credentials, whose helpers spawn subprocesses, out of a module
    # that exists to avoid spawning.
    declared = _workspace_dependency("gix")
    assert (
        isinstance(declared, dict) and declared.get("default-features") is False
    ), "the gix pin must disable default features"
    comment = _comment_above(_CLI_CARGO.read_text(), "jj-lib = ")
    assert "default-features" in comment, (
        f"the VCS pin comment no longer explains default-features: {comment}"
    )


def test_the_uluru_licence_exception_keeps_its_comment() -> None:
    comment = _comment_above(_DENY.read_text(), "[[licenses.exceptions]]")
    assert comment, "the uluru licence exception has lost its comment"
    assert "0188" in comment, (
        f"the uluru exception's comment must cite its work item: {comment}"
    )
