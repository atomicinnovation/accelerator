"""The names `bin/accelerator` writes into a cache root, against the shapes the
launcher's root prune recognises.

The prune keeps any name it does not recognise, so a bootstrap rename would
not fail loudly: its residue would quietly accumulate again. These tests read
each template out of the shell and hold it to the shape the launcher's
grammar in `retention/root_entries.rs` expects.
"""

import re
from pathlib import Path

from tasks.shared.targets import ALIASES

_REPO_ROOT = Path(__file__).resolve().parents[3]
_BOOTSTRAP = (_REPO_ROOT / "bin/accelerator").read_text()
_GRAMMAR = (
    _REPO_ROOT
    / "cli/launcher/src/launch/outbound/resolve/retention/root_entries.rs"
).read_text()

_PID = "4242"
_PLATFORM = "linux-x64"
_DIGEST = "0123456789abcdef" * 4

_LAUNCHER_STAGING = re.compile(r"\.tmp-launcher-[1-9][0-9]*(\.minisig)?")
_SHIM_STAGING = re.compile(
    r"accelerator-verify-(?P<platform>[a-z0-9-]+)-[0-9a-f]{64}"
    r"\.staging\.[1-9][0-9]*"
)
_PROBE = re.compile(r"\.accelerator-probe-[0-9]+(-[0-9]+)?")
_BOOTSTRAP_LOCK = re.compile(r"\.accelerator-lock-(?P<platform>[a-z0-9-]+)")


def _assignment(variable: str) -> str:
    match = re.search(rf'^\s*{variable}="([^"]+)"', _BOOTSTRAP, re.MULTILINE)
    assert match, f"bin/accelerator no longer assigns {variable}"
    return match.group(1)


def _expanded(template: str) -> str:
    variables = {
        "$$": _PID,
        "${cache_dir}/": "",
        "$1/": "",
        "${platform}": _PLATFORM,
        "${shim_digest}": _DIGEST,
        "${shim}": f"accelerator-verify-{_PLATFORM}-{_DIGEST}",
        "${tmp_bin}": f".tmp-launcher-{_PID}",
    }
    for variable, value in variables.items():
        template = template.replace(variable, value)
    assert "$" not in template, f"unexpanded variable in {template}"
    return template


def _grammar_constant(name: str) -> str:
    match = re.search(rf'const {name}: &str = "([^"]+)";', _GRAMMAR)
    assert match, f"root_entries.rs no longer defines {name}"
    return match.group(1)


def test_the_launcher_staging_names_have_the_recognised_shape() -> None:
    for variable in ("tmp_bin", "tmp_sig"):
        name = _expanded(_assignment(variable))
        assert _LAUNCHER_STAGING.fullmatch(name), name
    assert _grammar_constant("LAUNCHER_STAGING_PREFIX") == "launcher-"


def test_the_shim_staging_name_has_the_recognised_shape() -> None:
    name = _expanded(_assignment("shim_tmp"))
    match = _SHIM_STAGING.fullmatch(name)
    assert match, name
    assert match.group("platform") in ALIASES
    assert _grammar_constant("SHIM_STAGING_INFIX") == ".staging."
    assert _grammar_constant("SHIM_PREFIX") == "accelerator-verify-"


def test_the_probe_name_has_the_recognised_shape() -> None:
    name = _expanded(_assignment("probe"))
    assert _PROBE.fullmatch(name), name
    assert _grammar_constant("PROBE_PREFIX") == ".accelerator-probe-"


def test_the_bootstrap_lock_name_has_the_recognised_shape() -> None:
    name = _expanded(_assignment("lock_dir"))
    match = _BOOTSTRAP_LOCK.fullmatch(name)
    assert match, name
    assert match.group("platform") in ALIASES
    assert _grammar_constant("BOOTSTRAP_LOCK_PREFIX") == ".accelerator-lock-"


def test_the_lock_pid_file_holds_the_pid_and_a_newline() -> None:
    assert re.search(
        r"""printf '%s\\n' "\$\$" >"\$\{lock_dir\}/pid\"""", _BOOTSTRAP
    ), "the bootstrap no longer writes its lock pid as printf '%s\\n' \"$$\""
