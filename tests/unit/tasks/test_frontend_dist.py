"""Guards that the built visualiser SPA stays generated, never committed.

`cli/visualiser/frontend/dist/` was both tracked and matched by the nested
`cli/visualiser/frontend/.gitignore`. Under jj that combination silently
destroys the tracked copy: Vite empties the directory before rewriting it, a
snapshot taken while it is empty records the deletions, and the recreated files
are ignored so nothing re-tracks them. The next commit — on any branch, for any
reason — drops the SPA that `embed-dist` bakes into the release binary.

Untracking it removes the contradiction. These tests pin the two properties
that make that safe: the ignore rule survives, and every task that compiles the
embedding server builds the frontend first, so nothing depends on a committed
copy.

The walk is deliberately VCS-agnostic — `git ls-files` is blind in a jj
workspace, which would make a tracked-file assertion vacuous rather than
failing (see `tasks/shared/sources.py`).
"""

import tomllib
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MISE_TOML = REPO_ROOT / "mise.toml"
FRONTEND = "cli/visualiser/frontend"
DIST = f"{FRONTEND}/dist"

# Tasks that compile the server with the default `embed-dist` feature, or run
# against the built SPA. Each must build the frontend first now that no copy is
# committed. `build:frontend:stub` counts only for the lint-only compiles: it
# writes a placeholder index.html to satisfy build.rs, not a real bundle.
_NEEDS_REAL_BUILD = [
    "build:server:release",
    "build:server:cross-compile",
    "test:unit:visualiser",
    "test:e2e:visualiser",
]

# Tasks that only *compile* the server, so the placeholder index.html is
# enough. Measured from a dist-less tree: each of these fails in `build.rs`
# without it, which is what a fresh clone now looks like. `test:unit:cli` and
# the two cargo-deny tasks are deliberately absent — measured to pass without
# any dist at all.
_NEEDS_DIST_STUB = [
    "lint:cli:check",
    "lint:cli:fix",
    "pup:check",
    "test:integration:pup",
    "lint:server:check",
    "lint:server:fix",
]


@pytest.fixture
def mise() -> dict:
    return tomllib.loads(MISE_TOML.read_text())


def _task_depends(mise: dict, task: str) -> list[str]:
    return mise["tasks"][task].get("depends", [])


def test_the_built_spa_is_gitignored() -> None:
    import pathspec

    nested = (REPO_ROOT / FRONTEND / ".gitignore").read_text()
    spec = pathspec.GitIgnoreSpec.from_lines(nested.splitlines())
    for relative in ("dist/index.html", "dist/assets/index-abc123.js"):
        assert spec.match_file(relative), (
            f"{FRONTEND}/{relative} must stay ignored — tracking generated "
            f"output here is what let a rebuild delete the committed SPA"
        )


def test_every_font_in_dist_is_reproducible_from_public() -> None:
    # The fonts were the subtle half of the untracking: eleven of the fourteen
    # committed dist/ entries were fonts, and dropping them would have lost the
    # only copy had Vite not been sourcing them from public/. It is, so
    # public/fonts/ is the tracked original and dist/fonts/ pure output.
    public = REPO_ROOT / FRONTEND / "public/fonts"
    assert public.is_dir(), f"{FRONTEND}/public/fonts must be the font source"
    assert any(p.suffix == ".woff2" for p in public.iterdir()), (
        f"{FRONTEND}/public/fonts holds no fonts — dist/fonts would then be "
        f"the only copy, and it is no longer in version control"
    )

    built = REPO_ROOT / DIST / "fonts"
    if not built.is_dir():
        # A built tree is not guaranteed here: test:unit:tasks carries no
        # build:frontend edge. The assertions above are the load-bearing half
        # and always run; the comparison below is a bonus when one exists.
        return

    missing = {p.name for p in built.iterdir()} - {
        p.name for p in public.iterdir()
    }
    assert not missing, (
        f"dist/fonts carries files absent from public/fonts ({sorted(missing)})"
        f" — those exist only as build output and are no longer in version "
        f"control"
    )


@pytest.mark.parametrize("task", _NEEDS_REAL_BUILD)
def test_embedding_tasks_build_the_frontend_first(mise, task) -> None:
    assert "build:frontend" in _task_depends(mise, task), (
        f"{task} must depend on build:frontend — no dist/ is committed, so "
        f"the SPA it embeds or serves would be stale or absent"
    )


@pytest.mark.parametrize("task", _NEEDS_DIST_STUB)
def test_compile_only_tasks_gate_on_the_dist_stub(mise, task) -> None:
    depends = _task_depends(mise, task)
    assert "build:frontend:stub" in depends or "build:frontend" in depends, (
        f"{task} compiles the server with embed-dist, whose build.rs asserts "
        f"dist/index.html exists. With no committed dist/ it fails on a fresh "
        f"clone unless it gates on build:frontend:stub"
    )
