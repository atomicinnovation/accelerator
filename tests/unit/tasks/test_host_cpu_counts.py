import re

from invoke import Context

from tasks.host import cpu_counts


def _report(mocker, capsys, node_parallelism="12\n"):
    context = mocker.MagicMock(spec=Context)
    context.run.return_value.stdout = node_parallelism

    cpu_counts(context)

    return context, capsys.readouterr().out


def _count(report, label):
    match = re.search(rf"^{re.escape(label)}: (\S+)$", report, re.MULTILINE)
    assert match, f"{label} missing from:\n{report}"
    return match.group(1)


def test_it_reports_the_host_counts_that_size_python_workers(mocker, capsys):
    _, report = _report(mocker, capsys)

    for label in (
        "physical cores (psutil)",
        "logical CPUs (psutil)",
        "usable CPUs (os.process_cpu_count)",
    ):
        assert int(_count(report, label)) >= 1


def test_it_reports_the_parallelism_node_gives_playwright(mocker, capsys):
    context, report = _report(mocker, capsys, node_parallelism="12\n")

    assert _count(report, "available parallelism (Node)") == "12"
    assert "availableParallelism()" in context.run.call_args.args[0]


def test_an_unknown_physical_count_is_reported_as_unknown(mocker, capsys):
    mocker.patch("tasks.host.psutil.cpu_count", return_value=None)

    _, report = _report(mocker, capsys)

    assert _count(report, "physical cores (psutil)") == "unknown"
