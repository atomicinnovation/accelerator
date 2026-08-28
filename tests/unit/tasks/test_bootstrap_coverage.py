"""Guards for the extensionless bootstrap entry point.

`bin/accelerator` has no `.sh` extension, so it can silently escape the two
independent shell-source discovery mechanisms and, with them, the bash-3.2
floor. It is also one half of the plugin's trust root, which must stay
byte-identical to what the launcher embeds. These tests pin both.
"""

import re
from pathlib import Path

from tasks.shared.paths import vendored_shim_path
from tasks.shared.sources import shell_sources
from tasks.shared.targets import ALIASES

_REPO_ROOT = Path(__file__).resolve().parents[3]
_BOOTSTRAP = "bin/accelerator"
_KEY = "keys/accelerator-release.pub"
_BUILD_RS = _REPO_ROOT / "cli/launcher/build.rs"
_BOOTSTRAP_SRC = _REPO_ROOT / "bin/accelerator"
_PLUGIN_ROOT = "ACCELERATOR_PLUGIN_ROOT"
_PLUGIN_ROOT_READER = _REPO_ROOT / "cli/config-adapters/src/store.rs"


def test_bootstrap_is_in_the_shfmt_and_shellcheck_discovery() -> None:
    assert _BOOTSTRAP in shell_sources()


def test_bootstrap_is_in_the_bashisms_discovery() -> None:
    # The Python bashisms task scans the same discovered set as shfmt and
    # shellcheck; the extensionless bootstrap must appear in it.
    assert _BOOTSTRAP in shell_sources()


def test_bootstrap_is_an_executable_entrypoint() -> None:
    import os
    import stat

    mode = (_REPO_ROOT / _BOOTSTRAP).stat().st_mode
    assert mode & stat.S_IXUSR, "bin/accelerator must be executable (0755)"
    assert os.access(_REPO_ROOT / _BOOTSTRAP, os.X_OK)


def test_launcher_and_bootstrap_reference_the_same_committed_key() -> None:
    # Both trust anchors must point at the one committed key file.
    assert _KEY.rsplit("/", 1)[-1] in _BUILD_RS.read_text()
    assert _KEY in _BOOTSTRAP_SRC.read_text()
    assert (_REPO_ROOT / _KEY).is_file()


def test_bootstrap_exports_the_one_plugin_root_the_launcher_reads() -> None:
    # A one-sided rename otherwise surfaces as a missing sentinel deep in an
    # integration suite rather than here, in seconds. Exact equality on both
    # sides also catches a second, transitional export left behind.
    #
    # `config_adapters::plugin_root_from_env` is the one production call site
    # that reads the variable — every composition root (the launcher, the
    # visualiser server) calls through it rather than reading the
    # environment itself, so it's the only source this test pins against the
    # bootstrap's export.
    exported = set(
        re.findall(
            r"^export ([A-Z0-9_]*PLUGIN_ROOT)=",
            _BOOTSTRAP_SRC.read_text(),
            re.MULTILINE,
        )
    )
    assert exported == {_PLUGIN_ROOT}, (
        f"the bootstrap exports {sorted(exported)}, not just {_PLUGIN_ROOT}"
    )
    read = set(
        re.findall(
            r'var_os\("([A-Z0-9_]*PLUGIN_ROOT)"\)',
            _PLUGIN_ROOT_READER.read_text(),
        )
    )
    assert read == {_PLUGIN_ROOT}, (
        f"{_PLUGIN_ROOT_READER.name} reads {sorted(read)}, not {_PLUGIN_ROOT}"
    )


def test_the_cache_dir_helpers_keep_the_names_the_traces_assert_on() -> None:
    # The entrypoint trace cases match these tokens in a `bash -x` trace. A
    # rename would otherwise fail only after a cargo build and a full
    # fetch-verify-cache round trip, reporting "probe not entered" rather than
    # "the name moved" — and would silently void the warm-path negative
    # assertion in the meantime.
    for name in ("ensure_dir", "probe_exec_capable"):
        assert name in _BOOTSTRAP_SRC.read_text(), (
            f"{name} is asserted on by name in the entrypoint trace cases"
        )


def test_the_committed_vendored_shims_are_not_gitignored() -> None:
    # The staged-shim ignore pattern is digest-anchored; a `-*-*` form would
    # also match these four and block `git add` after a shim refresh.
    import pathspec

    gitignore = (_REPO_ROOT / ".gitignore").read_text()
    spec = pathspec.GitIgnoreSpec.from_lines(gitignore.splitlines())
    for alias in ALIASES:
        shim = vendored_shim_path(alias)
        relative = shim.relative_to(_REPO_ROOT).as_posix()
        assert not spec.match_file(relative), (
            f"{relative} must stay committable"
        )
        assert shim.is_file(), f"{relative} must be committed"
