import os

import psutil
from invoke import Context, task

_NODE_AVAILABLE_PARALLELISM = (
    "node -p \"require('node:os').availableParallelism()\""
)


def _shown(count: int | None) -> str:
    return "unknown" if count is None else str(count)


@task(name="cpu-counts")
def cpu_counts(context: Context) -> None:
    """Report the CPU counts that size the parallel test workers.

    pytest-xdist's `-n auto` takes psutil's physical count, and Playwright
    sizes its workers from Node's available parallelism.
    """
    node = context.run(_NODE_AVAILABLE_PARALLELISM, hide=True)
    counts = {
        "physical cores (psutil)": psutil.cpu_count(logical=False),
        "logical CPUs (psutil)": psutil.cpu_count(),
        "usable CPUs (os.process_cpu_count)": os.process_cpu_count(),
        "available parallelism (Node)": int(node.stdout),
    }
    for label, count in counts.items():
        print(f"{label}: {_shown(count)}")
