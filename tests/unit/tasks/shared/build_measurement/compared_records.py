from tasks.shared.build_measurement.measured_build import HostSettings
from tasks.shared.build_measurement.timeline import CargoUnits
from tasks.shared.build_measurement.tracked_processes import IDE
from tests.unit.tasks.shared.build_measurement.doubles import (
    measured_build,
    run_figures,
    task_figures,
)

NONE_RUN_ID = "build_2026-11-02T091500Z_engywook_warm-no-op_none"
DEVTOOLS_RUN_ID = "build_2026-11-02T101200Z_engywook_warm-no-op_devtools"
OS_VERSION_ARGV = ("sw_vers", "-productVersion")


def processes(syspolicyd, ide, fsnotifier, xprotect=None):
    return {
        "syspolicyd": syspolicyd,
        "XprotectService": xprotect,
        IDE: ide,
        "fsnotifier": fsnotifier,
    }


def record(
    *,
    wall_clocks=(303.3, 309.4, 317.0),
    process_cpu=None,
    tasks=None,
    host_settings=HostSettings.NONE,
    run_id=NONE_RUN_ID,
    **overrides,
):
    per_run_tasks = tasks or [{"build:cli": task_figures()}] * len(wall_clocks)
    per_run_cpu = process_cpu or [
        processes(10.0, 100.0, 5.0) for _ in wall_clocks
    ]
    runs = [
        run_figures(
            index,
            wall_clock=wall_clock,
            tasks=run_tasks,
            process_cpu=cpu,
        )
        for index, (wall_clock, run_tasks, cpu) in enumerate(
            zip(wall_clocks, per_run_tasks, per_run_cpu, strict=True),
            start=1,
        )
    ]
    return measured_build(
        runs=runs, host_settings=host_settings, run_id=run_id, **overrides
    )


def with_unit_cli(durations, nextest, units=None):
    return [
        {
            "build:cli": task_figures(duration=2.0),
            "test:unit:cli": task_figures(
                duration=duration,
                nextest=phase,
                units=units or CargoUnits(compiling=61, checking=12),
            ),
        }
        for duration, phase in zip(durations, nextest, strict=True)
    ]


def worked_example_previous():
    return record(
        wall_clocks=(303.3, 309.4, 317.0),
        process_cpu=[
            processes(49.0, 295.0, 12.0),
            processes(51.3, 301.0, 12.4),
            processes(53.0, 305.0, 12.9),
        ],
        tasks=with_unit_cli((290.0, 297.1, 300.0), (145.0, 149.2, 152.0)),
    )


def worked_example_current():
    return record(
        wall_clocks=(258.0, 262.0, 266.0),
        process_cpu=[
            processes(11.0, 294.0, 11.8),
            processes(12.0, 296.4, 12.1),
            processes(13.0, 299.0, 12.5),
        ],
        tasks=with_unit_cli((248.0, 251.0, 255.0), (115.0, 118.5, 121.0)),
        host_settings=HostSettings.DEVTOOLS,
        run_id=DEVTOOLS_RUN_ID,
    )


def cpu_records(previous_cpu, current_cpu, **current_overrides):
    previous = record(process_cpu=[processes(*each) for each in previous_cpu])
    current = record(
        process_cpu=[processes(*each) for each in current_cpu],
        **{
            "host_settings": HostSettings.DEVTOOLS,
            "run_id": DEVTOOLS_RUN_ID,
            **current_overrides,
        },
    )
    return previous, current
