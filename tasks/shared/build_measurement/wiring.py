import datetime as dt
import platform
import socket
import subprocess
from pathlib import Path

from tasks.shared.build_measurement.build_group import BuildGroupFile
from tasks.shared.build_measurement.build_runner import SubprocessBuildRunner
from tasks.shared.build_measurement.errors import (
    TaskGraphError,
    UnmeasurableDirectoryError,
)
from tasks.shared.build_measurement.history import GitHistory, JjHistory
from tasks.shared.build_measurement.provenance import History
from tasks.shared.build_measurement.samplers import (
    PsProcessSampler,
    StreamSampler,
)
from tasks.shared.build_measurement.session import BuildPorts
from tasks.shared.build_measurement.session_lock import WorkspaceSessionLock
from tasks.shared.build_measurement.tracked_processes import PS_ARGV
from tasks.shared.clock import Clock
from tasks.shared.diagnostics import ambient_diagnostic_runner
from tasks.shared.processes import PsutilProcessOps
from tasks.shared.repository import NotARepositoryError, repository_kind

_IOSTAT_ARGV = ("iostat", "-w", "2")
_MISE_TASKS_ARGV = ("mise", "tasks", "--json")


def history_of(root: Path) -> History:
    try:
        kind = repository_kind(root)
    except NotARepositoryError as error:
        raise UnmeasurableDirectoryError(
            f"measure:build needs a jj or git checkout; {root} is neither"
        ) from error
    return JjHistory(root) if kind.is_jj else GitHistory(root)


def _mise_tasks_json(root: Path) -> str:
    completed = subprocess.run(
        _MISE_TASKS_ARGV,
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        raise TaskGraphError(
            f"{' '.join(_MISE_TASKS_ARGV)} failed: {completed.stderr.strip()}"
        )
    return completed.stdout


def _progress(line: str) -> None:
    print(line, flush=True)


def real_ports(root: Path) -> BuildPorts:
    clock = Clock()
    procs = PsutilProcessOps()
    build_group = BuildGroupFile(root, procs)
    return BuildPorts(
        system=platform.system,
        build=SubprocessBuildRunner(root, clock, procs, build_group),
        cpu_sampler=StreamSampler(_IOSTAT_ARGV, clock),
        process_sampler=PsProcessSampler(
            PS_ARGV, clock, interval=1.0, tick_timeout=5.0
        ),
        history=history_of(root),
        diagnostics=ambient_diagnostic_runner,
        kernel_hostname=socket.gethostname,
        mise_tasks=lambda: _mise_tasks_json(root),
        clock=clock,
        utc_now=lambda: dt.datetime.now(tz=dt.UTC),
        progress=_progress,
        session_lock=WorkspaceSessionLock(root).held,
        build_group=build_group,
    )
