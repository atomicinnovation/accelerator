"""Black-box characterisation of the built cli binaries.

A case runs a compiled binary in a hermetic git or jj repository and compares
its exit status, stdout, stderr and refused release fetches against a committed
golden. Nothing here calls a Rust API, so a refactor that reshapes the crates
cannot reshape the suite.
"""

import os
import re
import shutil
import subprocess
import threading
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, replace
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

from tasks.shared.dev_builds import DEV_BUILDS, DevBinary
from tests.support.artefacts import CLI_TARGET_DIR, claim_artefact
from tests.support.tools import in_ci

GOLDENS = Path(__file__).resolve().parents[1] / "characterisation" / "goldens"

IDENTITY_NAME = "Fixture"
IDENTITY_EMAIL = "fixture@example.com"

# Provider credentials a developer shell commonly exports. Any one of them
# leaking into a case would bypass the consent keys the suite pins.
_SCRUBBED_PREFIXES = ("ACCELERATOR_", "GIT_", "JJ_")
_SCRUBBED = frozenset(
    {
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GH_HOST",
        "GH_ENTERPRISE_TOKEN",
        "LINEAR_API_KEY",
        "JIRA_API_TOKEN",
        "OPENALEX_API_KEY",
        "CLAUDE_PLUGIN_ROOT",
        "CLAUDE_PROJECT_DIR",
        "RUST_LOG",
        "RUST_BACKTRACE",
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    }
)


def _inherited() -> dict[str, str]:
    return {
        key: value
        for key, value in os.environ.items()
        if key not in _SCRUBBED and not key.startswith(_SCRUBBED_PREFIXES)
    }


class Binaries:
    """Claimed copies of the debug build the characterisation group produced."""

    def __init__(self, binaries: Iterable[DevBinary]) -> None:
        self._paths = {
            binary.name: claim_artefact(CLI_TARGET_DIR / "debug" / binary.name)
            for binary in binaries
        }

    @classmethod
    def characterisation(cls) -> Binaries:
        return cls(DEV_BUILDS["characterisation"])

    def __getitem__(self, name: str) -> Path:
        return self._paths[name]

    @property
    def launcher(self) -> Path:
        return self._paths["accelerator"]

    def subbinary(self, token: str) -> Path:
        return self._paths[f"accelerator-{token}"]


@dataclass
class Hermetic:
    """An isolated HOME, VCS config and ceiling for every case's processes."""

    root: Path

    @classmethod
    def rooted_at(cls, root: Path) -> Hermetic:
        root = root.resolve()
        hermetic = cls(root)
        hermetic.home.mkdir(parents=True, exist_ok=True)
        hermetic.config_home.mkdir(parents=True, exist_ok=True)
        hermetic.jj_config.write_text(
            f'[user]\nname = "{IDENTITY_NAME}"\n'
            f'email = "{IDENTITY_EMAIL}"\n'
            '[ui]\npaginate = "never"\ncolor = "never"\n'
            '[snapshot]\nauto-track = "all()"\n'
        )
        return hermetic

    @property
    def home(self) -> Path:
        return self.root / "home"

    @property
    def config_home(self) -> Path:
        return self.root / "xdg"

    @property
    def jj_config(self) -> Path:
        return self.root / "jj.toml"

    def environment(self, overlay: Mapping[str, str] = {}) -> dict[str, str]:
        return {
            **_inherited(),
            "HOME": str(self.home),
            "XDG_CONFIG_HOME": str(self.config_home),
            "JJ_CONFIG": str(self.jj_config),
            "GIT_CEILING_DIRECTORIES": str(self.root),
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": "/dev/null",
            "GIT_AUTHOR_NAME": IDENTITY_NAME,
            "GIT_AUTHOR_EMAIL": IDENTITY_EMAIL,
            "GIT_COMMITTER_NAME": IDENTITY_NAME,
            "GIT_COMMITTER_EMAIL": IDENTITY_EMAIL,
            "GIT_AUTHOR_DATE": "2026-01-01T00:00:00Z",
            "GIT_COMMITTER_DATE": "2026-01-01T00:00:00Z",
            "LC_ALL": "C",
            "TZ": "UTC",
            "ACCELERATOR_LOG": "off",
            **overlay,
        }

    def run(
        self, args: list[str], cwd: Path, overlay: Mapping[str, str] = {}
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            args,
            cwd=cwd,
            env=self.environment(overlay),
            capture_output=True,
            text=True,
            check=False,
            stdin=subprocess.DEVNULL,
            timeout=120,
        )

    def tool(self, args: list[str], cwd: Path) -> str:
        completed = self.run(args, cwd)
        if completed.returncode != 0:
            message = (
                f"{' '.join(args)} failed in {cwd}:\n"
                f"{completed.stdout}{completed.stderr}"
            )
            raise AssertionError(message)
        return completed.stdout


def assert_outside_any_repository(path: Path) -> None:
    """Refuse a fixture root nested in a real repository.

    gix ignores GIT_CEILING_DIRECTORIES, so a case under one would silently
    characterise the enclosing checkout instead of its own fixture.
    """
    for ancestor in path.resolve().parents:
        for marker in (".git", ".jj"):
            if (ancestor / marker).exists():
                pytest.fail(
                    f"fixture root {path} lies inside the repository at "
                    f"{ancestor}; characterisation needs an unenclosed "
                    "temp root"
                )


class Repository:
    """A hermetic fixture repository of one VCS kind."""

    def __init__(self, kind: str, root: Path, hermetic: Hermetic) -> None:
        self.kind = kind
        self.root = root
        self.hermetic = hermetic

    @classmethod
    def create(cls, kind: str, root: Path, hermetic: Hermetic) -> Repository:
        root.mkdir(parents=True, exist_ok=True)
        repository = cls(kind, root, hermetic)
        if kind == "git":
            repository.git("init", "--quiet", "--initial-branch=main")
            repository.git("config", "user.name", IDENTITY_NAME)
            repository.git("config", "user.email", IDENTITY_EMAIL)
        elif kind == "jj":
            repository.jj("git", "init")
        elif kind == "colocated-jj":
            repository.jj("git", "init", "--colocate")
        else:
            raise ValueError(f"unknown repository kind {kind!r}")
        return repository

    @property
    def is_git(self) -> bool:
        return self.kind == "git"

    def git(self, *args: str) -> str:
        return self.hermetic.tool(
            ["git", "-c", "commit.gpgsign=false", *args], self.root
        )

    def jj(self, *args: str) -> str:
        return self.hermetic.tool(
            ["jj", "--color=never", "--no-pager", *args], self.root
        )

    def revisions(self) -> list[Mask]:
        """Name this repository's own commits, read without a snapshot."""
        if self.is_git:
            head = self.hermetic.run(
                ["git", "rev-parse", "--verify", "--quiet", "HEAD"], self.root
            ).stdout.strip()
            return [Mask(head, "HEAD")] if head else []
        return [
            Mask(self._jj_commit(revision), name)
            for revision, name in (("@", "WORKING_COPY"), ("@-", "PARENT"))
        ]

    def _jj_commit(self, revision: str) -> str:
        return self.jj(
            "--ignore-working-copy",
            "log",
            "--no-graph",
            "-r",
            revision,
            "-T",
            "commit_id",
        ).strip()

    def write(self, relative: str, content: str) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def commit(self, message: str = "fixture") -> None:
        """Record every present, non-ignored file in a commit."""
        if self.is_git:
            self.git("add", "--all")
            self.git("commit", "--quiet", "--allow-empty", "-m", message)
        else:
            self.jj("commit", "-m", message)

    def track(self, relative: str) -> None:
        self.commit(f"track {relative}")

    def untrack(self, relative: str) -> None:
        """Keep `relative` on disk while the repository ignores it."""
        ignore = self.root / Path(relative).parent / ".gitignore"
        ignored = ignore.read_text() if ignore.exists() else ""
        ignore.write_text(ignored + f"{Path(relative).name}\n")
        self.commit(f"ignore {relative}")


@dataclass(frozen=True)
class Hit:
    method: str
    path: str


Responder = Callable[[str, str, bytes], tuple[int, str]]


def _not_found(method: str, path: str, body: bytes) -> tuple[int, str]:
    return 404, ""


class Loopback:
    """A stdlib HTTP server on 127.0.0.1 that records every request."""

    def __init__(self, responder: Responder = _not_found) -> None:
        self.hits: list[Hit] = []
        self.responder = responder
        loopback = self

        class Handler(BaseHTTPRequestHandler):
            def _answer(self) -> None:
                length = int(self.headers.get("Content-Length") or 0)
                body = self.rfile.read(length) if length else b""
                loopback.hits.append(Hit(self.command, self.path))
                status, text = loopback.responder(self.command, self.path, body)
                payload = text.encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

            # The stdlib dispatches each request to a handler by this name.
            do_GET = do_POST = do_PUT = do_PATCH = do_HEAD = _answer  # noqa: N815

            def log_message(self, message_format: str, *args: object) -> None:
                return

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(
            target=self._server.serve_forever, daemon=True
        )
        self._thread.start()

    def serve(self, responder: Responder) -> None:
        self.responder = responder

    def refuse_everything(self) -> None:
        self.responder = _not_found

    @property
    def url(self) -> str:
        host, port = self._server.server_address[:2]
        return f"http://{host}:{port}"

    def close(self) -> None:
        self._server.shutdown()
        self._server.server_close()


@dataclass(frozen=True)
class Run:
    code: int
    stdout: str
    stderr: str
    loopback_requests: int
    observations: tuple[str, ...] = ()

    def with_observation(self, observation: str) -> Run:
        return replace(self, observations=(*self.observations, observation))

    def render(self) -> str:
        observed = "".join(f"{line}\n" for line in self.observations)
        return (
            f"exit: {self.code}\n"
            f"loopback_requests: {self.loopback_requests}\n"
            f"--- stdout\n{_terminated(self.stdout)}"
            f"--- stderr\n{_terminated(self.stderr)}"
            + (f"--- observed\n{observed}" if observed else "")
        )


def _terminated(stream: str) -> str:
    """Close an unterminated stream so the next header starts its own line.

    A missing final newline is itself behaviour, so it is marked rather than
    silently added.
    """
    if not stream or stream.endswith("\n"):
        return stream
    return f"{stream}\n<no newline at end>\n"


@dataclass(frozen=True)
class Mask:
    """Replace a volatile substring with a stable `<NAME>` placeholder."""

    pattern: str
    name: str
    literal: bool = True

    def apply(self, text: str) -> str:
        if not self.pattern:
            return text
        if self.literal:
            return text.replace(self.pattern, f"<{self.name}>")
        return re.sub(self.pattern, f"<{self.name}>", text)


NOW = Mask(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\+00:00|Z)", "NOW", literal=False)
FILENAME_STAMP = Mask(
    r"\d{4}-\d\d-\d\d(_\d\d-\d\d-\d\d|-\d{6})?", "STAMP", literal=False
)
REVISION = Mask(r"\b[0-9a-f]{40}\b", "REVISION", literal=False)


def masked(text: str, masks: Iterable[Mask]) -> str:
    for mask in masks:
        text = mask.apply(text)
    return text


def golden_path(node_name: str, module: str) -> Path:
    stem = re.sub(r"[^A-Za-z0-9_.-]+", "-", node_name).strip("-")
    return GOLDENS / module / f"{stem}.txt"


def assert_matches_golden(golden: Path, rendered: str) -> None:
    if os.environ.get("UPDATE_GOLDEN") == "1":
        if in_ci():
            pytest.fail("UPDATE_GOLDEN is refused in CI")
        golden.parent.mkdir(parents=True, exist_ok=True)
        golden.write_text(rendered)
        return
    if not golden.exists():
        pytest.fail(
            f"missing golden {golden.relative_to(GOLDENS.parent)}; "
            "rerun with UPDATE_GOLDEN=1 and review the new file"
        )
    assert rendered == golden.read_text()


@dataclass(frozen=True)
class CollectedCase:
    case: tuple[str, tuple[tuple[str, str], ...]]
    vcs: str | None
    vcs_specific: bool
    runs_a_binary: bool = True


def vcs_parity_violations(cases: Iterable[CollectedCase]) -> list[str]:
    """Name every case that runs in only one of git and jj."""
    kinds: dict[tuple[str, tuple[tuple[str, str], ...]], set[str]] = {}
    for collected in cases:
        if collected.vcs_specific or not collected.runs_a_binary:
            continue
        kinds.setdefault(collected.case, set())
        if collected.vcs is not None:
            kinds[collected.case].add(collected.vcs)
    return sorted(
        f"{name}{dict(params) or ''}: runs in {sorted(found) or 'neither'}"
        for (name, params), found in kinds.items()
        if found != {"git", "jj"}
    )


def which(tool: str) -> str:
    found = shutil.which(tool)
    if found is None:
        pytest.fail(f"{tool} not on PATH")
    return found
