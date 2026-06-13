import shlex
import sys

from invoke import Context, Exit, task

from tasks.shared.dev.host_server import run_against_host_server
from tasks.shared.paths import FRONTEND, SERVER
from tasks.shared.playwright import (
    BROWSER_LOCALE,
    CHROMIUM_CHANNEL,
    E2E_LANG,
    PLAYWRIGHT_PLATFORM,
    playwright_image,
    resolved_playwright_version,
)

SERVER_BIN = SERVER / "target/debug/accelerator-visualiser"


@task
def visualiser(context: Context) -> None:
    """E2E tests for the visualiser (Playwright).

    Requires build.frontend and build.server to have run first.
    When invoked via `mise run test:e2e:visualiser`, those build tasks
    are declared as dependencies and run automatically.
    """
    context.run(
        f"npm --prefix {FRONTEND} run test:e2e",
        env={"ACCELERATOR_VISUALISER_BIN": str(SERVER_BIN)},
    )


def docker_visual_command(
    base_url: str, image: str, update: bool, cache_deps: bool = False
) -> str:
    """Assemble the `docker run` command for the visual-regression specs.

    Returned as a single ``shlex.join``-ed string so nested-quote bugs are
    designed out and the unit tests assert on discrete tokens.
    """
    pw = (
        "npx playwright test --config playwright.docker.config.ts "
        "--project visual-regression"
    )
    if update:
        pw += " --update-snapshots"
    # No locale-gen: C.UTF-8 is pre-generated in every Playwright image and the
    # rendering locale is Chromium's explicit en-US (see
    # tasks/shared/playwright.py).
    bootstrap = f"npm ci && {pw}"
    # Either an anonymous mask (default, ephemeral, always correct) or a named
    # volume keyed to the resolved Playwright version (opt-in --cache-deps) so a
    # cached node_modules can never outlive a Playwright bump.
    node_modules_volume = (
        f"pw-node-modules-{resolved_playwright_version()}:/work/node_modules"
        if cache_deps
        else "/work/node_modules"
    )
    args = [
        "docker", "run", "--rm",
        f"--platform={PLAYWRIGHT_PLATFORM}", "--ipc=host",
        "--add-host=host.docker.internal:host-gateway",
        # Mount only the frontend dir (least privilege): the container needs the
        # specs/config/baselines, not the whole repo. The host server runs
        # outside the container and is reached over BASE_URL, so .e2e-port is
        # never read inside it.
        "-v", f"{FRONTEND}:/work",
        # Mask node_modules so the container's Linux-native binaries never
        # clobber the host tree (anonymous by default; named iff --cache-deps).
        "-v", node_modules_volume,
        "-w", "/work",
        "-e", "CI=1",
        "-e", f"BASE_URL={base_url}",
        "-e", f"CHROMIUM_CHANNEL={CHROMIUM_CHANNEL}",
        "-e", f"PLAYWRIGHT_LOCALE={BROWSER_LOCALE}",
        "-e", f"LANG={E2E_LANG}",
        "-e", f"LC_ALL={E2E_LANG}",
        image,
        "bash", "-c", bootstrap,
    ]  # fmt: skip
    return shlex.join(args)


@task
def visualiser_docker(
    context: Context, update: bool = False, cache_deps: bool = False
) -> None:
    """Run the visual-regression specs in the pinned Playwright Docker image.

    Only Chromium is containerised: the committed baseline is a function of what
    Chromium renders, so the Rust server runs on the *host* (as in CI) and the
    container reaches it over `host.docker.internal`. `--network=host` is
    deliberately not used locally — under Colima it joins the Lima VM namespace,
    not the macOS host.

    `--update` regenerates the canonical Linux baseline set. `--cache-deps`
    persists node_modules in a named volume across runs to skip the (slow,
    emulated) `npm ci` each iteration; the default anonymous mask is used
    otherwise for correctness.
    """
    # Fail fast with an actionable message if Docker is unavailable, BEFORE
    # spending up to 60s bringing up the host server (and before leaving one
    # started). `docker info` is a cheap daemon-reachability probe.
    if context.run("docker info", hide=True, warn=True).failed:
        raise Exit(
            "Docker daemon not reachable — start Docker Desktop / Colima and "
            "retry. See skills/visualisation/visualise/frontend/README.md "
            "(Visual-Regression Baselines).",
            code=1,
        )

    # The mise tasks declare build:server:dev as a dependency, but a direct
    # `invoke` call (e.g. for --cache-deps) bypasses it — so check explicitly.
    if not SERVER_BIN.exists():
        raise Exit(
            f"Server binary not found at {SERVER_BIN} — run "
            "`mise run build:server:dev` first (or use the mise tasks).",
            code=1,
        )

    def on_ready(port: str) -> None:
        context.run(
            docker_visual_command(
                base_url=f"http://host.docker.internal:{port}",
                image=playwright_image(),
                update=update,
                cache_deps=cache_deps,
            ),
            # Live Docker/npm output locally; piped (no PTY) on the headless CI
            # runner so the container exit code is reliably propagated.
            pty=sys.stdout.isatty(),
        )

    run_against_host_server(server_bin=SERVER_BIN, on_ready=on_ready)
