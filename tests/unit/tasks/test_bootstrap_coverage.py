"""Guards for the extensionless bootstrap entry point.

`bin/accelerator` has no `.sh` extension, so it can silently escape the two
independent shell-source discovery mechanisms and, with them, the bash-3.2
floor. It is also one half of the plugin's trust root, which must stay
byte-identical to what the launcher embeds. These tests pin both.
"""

from pathlib import Path

from tasks.shared.sources import shell_sources

_REPO_ROOT = Path(__file__).resolve().parents[3]
_BOOTSTRAP = "bin/accelerator"
_KEY = "keys/accelerator-release.pub"
_BASHISMS = _REPO_ROOT / "scripts/lint-bashisms.sh"
_BUILD_RS = _REPO_ROOT / "cli/launcher/build.rs"
_BOOTSTRAP_SRC = _REPO_ROOT / "bin/accelerator"


def test_bootstrap_is_in_the_shfmt_and_shellcheck_discovery() -> None:
    # shfmt and shellcheck both consume shell_sources(); the extensionless
    # bootstrap is appended via _EXTRA_SHELL_SOURCES.
    assert _BOOTSTRAP in shell_sources()


def test_bootstrap_is_in_the_bashisms_discovery() -> None:
    # lint-bashisms.sh discovers via `git ls-files '*.sh'`, which never matches
    # an extensionless file, so it must add bin/accelerator explicitly.
    assert _BOOTSTRAP in _BASHISMS.read_text()


def test_bootstrap_is_an_executable_entrypoint() -> None:
    import os
    import stat

    mode = (_REPO_ROOT / _BOOTSTRAP).stat().st_mode
    assert mode & stat.S_IXUSR, "bin/accelerator must be executable (0755)"
    assert os.access(_REPO_ROOT / _BOOTSTRAP, os.X_OK)


def test_launcher_and_bootstrap_reference_the_same_committed_key() -> None:
    # The launcher embeds the key via build.rs and the bootstrap reads it at
    # runtime; both must point at the ONE committed file, so the two in-repo
    # trust anchors cannot drift.
    assert _KEY.rsplit("/", 1)[-1] in _BUILD_RS.read_text()
    assert _KEY in _BOOTSTRAP_SRC.read_text()
    assert (_REPO_ROOT / _KEY).is_file()
