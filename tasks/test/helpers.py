import os
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def accelerator_env(
    *, vcs_bin: bool = False, corpus_bin: bool = False
) -> dict[str, str]:
    """Return the ACCELERATOR_BIN/ACCELERATOR_PLUGIN_ROOT overlay.

    Repointed production shell scripts read config through
    ``"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config …``. This
    overlay is layered onto a child process via ``context.run(env=...)`` — it is
    *not* written into this process's environment — so it keeps a subtree's
    suites on the compiled binary rather than the signed-release bootstrap with
    no global side effect. ACCELERATOR_PLUGIN_ROOT is included for template
    lookup: the suites invoke the launcher directly, bypassing the bootstrap
    that would otherwise export it.

    The launcher itself is produced by the ``build:cli:dev`` mise dependency of
    the test tasks, so build ordering lives in the task graph rather than in an
    ad-hoc cargo call here. A caller-supplied
    ACCELERATOR_BIN/ACCELERATOR_PLUGIN_ROOT (a stub or a release build) is
    honoured.

    ``vcs_bin=True`` additionally sets ACCELERATOR_VCS_BIN, the launcher's
    dev-only ``accelerator vcs ...`` dispatch override — needed only by the
    vcs-detect launcher-dispatch guard (tests/integration/hooks), so it stays
    opt-in rather than joining every subtree's overlay.

    ``corpus_bin=True`` additionally sets ACCELERATOR_CORPUS_BIN, the
    launcher's dev-only ``accelerator corpus ...`` dispatch override — needed
    by suites that drive `accelerator corpus frontmatter validate`/`linkage
    extract` through the real launcher rather than the `corpus-cli` crate's
    own golden tests (which invoke the compiled binary directly and never go
    through dispatch).
    """
    repo = repo_root()
    default_bin = str(repo / "cli" / "target" / "debug" / "accelerator")
    env = {
        "ACCELERATOR_BIN": os.environ.get("ACCELERATOR_BIN", default_bin),
        "ACCELERATOR_PLUGIN_ROOT": os.environ.get(
            "ACCELERATOR_PLUGIN_ROOT", str(repo)
        ),
    }
    if vcs_bin:
        default_vcs_bin = str(
            repo / "cli" / "target" / "debug" / "accelerator-vcs"
        )
        env["ACCELERATOR_VCS_BIN"] = os.environ.get(
            "ACCELERATOR_VCS_BIN", default_vcs_bin
        )
    if corpus_bin:
        default_corpus_bin = str(
            repo / "cli" / "target" / "debug" / "accelerator-corpus"
        )
        env["ACCELERATOR_CORPUS_BIN"] = os.environ.get(
            "ACCELERATOR_CORPUS_BIN", default_corpus_bin
        )
    return env
