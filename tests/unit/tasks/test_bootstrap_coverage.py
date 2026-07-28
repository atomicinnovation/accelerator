"""Guards for the extensionless bootstrap entry point.

`bin/accelerator` has no `.sh` extension, so it can silently escape the two
independent shell-source discovery mechanisms and, with them, the bash-3.2
floor. It is also one half of the plugin's trust root, which must stay
byte-identical to what the launcher embeds. These tests pin both.
"""

from pathlib import Path

from tasks.shared.paths import vendored_shim_path
from tasks.shared.sources import shell_sources
from tasks.shared.targets import ALIASES

_REPO_ROOT = Path(__file__).resolve().parents[3]
_BOOTSTRAP = "bin/accelerator"
_KEY = "keys/accelerator-release.pub"
_BASHISMS = _REPO_ROOT / "scripts/lint-bashisms.sh"
_BUILD_RS = _REPO_ROOT / "cli/launcher/build.rs"
_BOOTSTRAP_SRC = _REPO_ROOT / "bin/accelerator"
_LAUNCHER_MAIN = _REPO_ROOT / "cli/launcher/src/main.rs"
_CACHE_ROOT = (
    _REPO_ROOT / "cli/launcher/src/launch/outbound/resolve/cache_root.rs"
)


def test_bootstrap_is_in_the_shfmt_and_shellcheck_discovery() -> None:
    assert _BOOTSTRAP in shell_sources()


def test_bootstrap_is_in_the_bashisms_discovery() -> None:
    # lint-bashisms.sh globs `*.sh`, which never matches an extensionless file.
    assert _BOOTSTRAP in _BASHISMS.read_text()


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


def test_bootstrap_exports_the_names_the_launcher_reads() -> None:
    # A one-sided rename otherwise surfaces as a missing sentinel deep in an
    # integration suite rather than here, in seconds.
    bootstrap = _BOOTSTRAP_SRC.read_text()
    exported = {
        name
        for name in ("ACCELERATOR_PLUGIN_ROOT", "CLAUDE_PLUGIN_ROOT")
        if f"export {name}=" in bootstrap
    }
    read = set()
    for source in (_LAUNCHER_MAIN, _CACHE_ROOT):
        text = source.read_text()
        read |= {
            name
            for name in ("ACCELERATOR_PLUGIN_ROOT", "CLAUDE_PLUGIN_ROOT")
            if f'var_os("{name}")' in text
        }
    assert read, "the launcher must read a plugin-root variable"
    assert read <= exported, (
        f"read but never exported: {sorted(read - exported)}"
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
