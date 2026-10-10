import tomllib

import pytest
import yaml

from tasks.shared.paths import REPO_ROOT

WORKFLOW = REPO_ROOT / ".github/workflows/runtime-pin-guard.yml"
GUARD_JOB = "guard-pins"


@pytest.fixture(scope="module")
def workflow():
    return yaml.safe_load(WORKFLOW.read_text())


@pytest.fixture(scope="module")
def job(workflow):
    return workflow["jobs"][GUARD_JOB]


def _step_using(job, action):
    return next(
        step
        for step in job["steps"]
        if str(step.get("uses", "")).split("@", 1)[0] == action
    )


def _guard_step(job):
    return next(
        step
        for step in job["steps"]
        if "vendor:guard-pins" in step.get("run", "")
    )


def test_it_runs_daily_and_on_dispatch(workflow):
    triggers = workflow[True]
    (schedule,) = triggers["schedule"]
    fields = schedule["cron"].split()
    assert len(fields) == 5
    assert fields[2:] == ["*", "*", "*"]
    assert "workflow_dispatch" in triggers


def test_only_the_job_holds_permissions_and_only_to_file_issues(workflow, job):
    assert workflow["permissions"] == {}
    assert job["permissions"] == {"contents": "read", "issues": "write"}


def test_runs_never_overlap(workflow):
    concurrency = workflow["concurrency"]
    assert concurrency["group"]
    assert concurrency["cancel-in-progress"] is False


def test_it_installs_the_whole_toolset_mise_run_would_install_anyway(job):
    mise = _step_using(job, "jdx/mise-action")
    assert mise["with"]["install"] is True
    assert "install_args" not in mise["with"]


def test_the_guard_step_opens_issues_under_the_dispatched_cap(job):
    run = _guard_step(job)["run"]
    assert "mise run vendor:guard-pins" in run
    assert "--open-issues" in run
    assert '--maximum-new-issues "$MAXIMUM_NEW_ISSUES"' in run


def test_the_guard_step_authenticates_with_the_workflow_token(job):
    env = _guard_step(job)["env"]
    assert env["GH_TOKEN"] == "${{ secrets.GITHUB_TOKEN }}"


def test_the_cap_comes_from_the_dispatch_input_defaulting_to_ten(workflow, job):
    cap = workflow[True]["workflow_dispatch"]["inputs"]["maximum-new-issues"]
    assert cap["default"] == 10
    env = _guard_step(job)["env"]
    assert env["MAXIMUM_NEW_ISSUES"] == (
        "${{ inputs.maximum-new-issues || 10 }}"
    )


def test_no_run_interpolates_an_expression(job):
    for step in job["steps"]:
        assert "${{" not in step.get("run", ""), step.get("name")


def test_the_guard_task_is_a_mise_task():
    mise = tomllib.loads((REPO_ROOT / "mise.toml").read_text())
    assert "vendor:guard-pins" in mise["tasks"]
