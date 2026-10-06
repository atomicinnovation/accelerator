import json
import os

import pytest

from tests.integration.support.characterisation import Mask

FRESH_TIMESTAMP = Mask(
    r"(?!2026-01-01T00:00:00Z)\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\+00:00|Z)",
    "NOW",
    literal=False,
)
DEAD_PID = "2147483632"
SENTINEL = "owner.0123456789abcdef"
CREATE_LOCK = "meta/work/.accelerator-work-create.lockdir"
ITEM = "meta/work/0001-existing.md"

_ITEM = """\
---
type: "work-item"
id: "0001"
title: "Existing"
date: "2026-01-01T00:00:00Z"
author: "Fixture"
status: "draft"
kind: "task"
priority: "medium"
last_updated: "2026-01-01T00:00:00Z"
last_updated_by: "Fixture"
schema_version: 1
---

# 0001: Existing
"""

_JIRA = (
    "---\nwork:\n  integration: jira\n"
    "jira:\n  site: acme\n  email: fixture@example.com\n"
    "  project_key: ENG\n---\n"
)

_SYNCED_ITEM = _ITEM.replace(
    'priority: "medium"\n', 'priority: "medium"\nexternal_id: "ENG-1"\n'
)

_TOKEN = {"ACCELERATOR_JIRA_TOKEN": "fixture-token"}


@pytest.fixture
def work(repository):
    repository.write(ITEM, _ITEM)
    repository.commit()
    return repository


def _work(run, binaries, repository, *args, env=None):
    return run(
        binaries.subbinary("work"), *args, cwd=repository.root, env=env or {}
    )


def _observe(result, repository, *relative):
    for path in relative:
        target = repository.root / path
        if target.is_dir():
            listing = sorted(p.name for p in target.iterdir())
            result = result.with_observation(f"{path}/ holds {listing}")
        elif target.exists():
            result = result.with_observation(
                f"{path}:\n{target.read_text().rstrip()}"
            )
        else:
            result = result.with_observation(f"{path} is absent")
    return result


def _plant_dead_lock(lockdir):
    lockdir.mkdir(parents=True)
    (lockdir / SENTINEL).write_text(DEAD_PID)


@pytest.fixture
def read_only(request):
    made = []

    def make(path):
        path.chmod(0o555)
        made.append(path)

    yield make
    for path in made:
        path.chmod(0o755)


@pytest.fixture
def root_masks(repository):
    return [Mask(str(repository.root), "REPOSITORY"), FRESH_TIMESTAMP]


def test_create_takes_the_author_from_the_repository(
    work, binaries, run, matches_golden, root_masks
):
    result = _work(run, binaries, work, "create", "Fresh item", "task", "high")

    matches_golden(
        _observe(result, work, "meta/work/0002-fresh-item.md", CREATE_LOCK),
        root_masks,
    )


def test_create_reclaims_a_lock_left_by_a_dead_process(
    work, binaries, run, matches_golden, root_masks
):
    _plant_dead_lock(work.root / CREATE_LOCK)

    result = _work(run, binaries, work, "create", "After crash", "bug", "low")

    matches_golden(
        _observe(result, work, "meta/work/0002-after-crash.md", CREATE_LOCK),
        root_masks,
    )


@pytest.mark.skipif(os.geteuid() == 0, reason="root ignores permission bits")
def test_create_under_a_read_only_work_directory(
    work, binaries, run, matches_golden, root_masks, read_only
):
    read_only(work.root / "meta/work")

    result = _work(run, binaries, work, "create", "Blocked", "task", "high")

    matches_golden(result, root_masks)


def test_update_on_a_dirty_working_copy(
    work, binaries, run, matches_golden, root_masks
):
    work.write(ITEM, _ITEM.replace("# 0001: Existing", "# 0001: Edited"))

    result = _work(run, binaries, work, "update", ITEM, "--set", "status=ready")

    matches_golden(_observe(result, work, ITEM), root_masks)


def test_update_reclaims_a_lock_left_by_a_dead_process(
    work, binaries, run, matches_golden, root_masks
):
    _plant_dead_lock(work.root / f"{ITEM}.lockdir")

    result = _work(
        run, binaries, work, "update", ITEM, "--set", "priority=high"
    )

    matches_golden(_observe(result, work, ITEM, f"{ITEM}.lockdir"), root_masks)


@pytest.mark.skipif(os.geteuid() == 0, reason="root ignores permission bits")
def test_update_under_a_read_only_work_directory(
    work, binaries, run, matches_golden, root_masks, read_only
):
    read_only(work.root / "meta/work")

    result = _work(run, binaries, work, "update", ITEM, "--set", "status=ready")

    matches_golden(result, root_masks)


def _baseline(repository, hash_value="0" * 64):
    baseline = {
        "timestamp": 1,
        "items": {
            "0001": {
                "remote_updated_at": "2026-06-01T00:00:00Z",
                "remote_hash": "rh",
                "local_hash": hash_value,
            }
        },
    }
    repository.write(
        ".accelerator/state/integrations/jira/last-sync.json",
        json.dumps(baseline) + "\n",
    )


@pytest.mark.parametrize("working_copy", ["clean", "dirty"])
def test_sync_preview_reports_working_copy_status(
    work, binaries, run, matches_golden, root_masks, working_copy
):
    work.write(".accelerator/config.md", _JIRA)
    work.commit()
    if working_copy == "dirty":
        work.write(ITEM, _ITEM.replace("Existing", "Edited locally"))

    result = _work(
        run,
        binaries,
        work,
        "sync",
        "--preview",
        "--push-only",
        env=_TOKEN,
    )

    matches_golden(result, root_masks)


@pytest.mark.parametrize("baseline", ["without-baseline", "with-baseline"])
def test_list_reports_sync_presence_without_credentials(
    work, binaries, run, matches_golden, root_masks, baseline
):
    work.write(".accelerator/config.md", _JIRA)
    work.write(ITEM, _SYNCED_ITEM)
    if baseline == "with-baseline":
        _baseline(work)
    work.commit()

    matches_golden(_work(run, binaries, work, "list"), root_masks)


def test_list_without_an_integration(
    work, binaries, run, matches_golden, root_masks
):
    matches_golden(_work(run, binaries, work, "list"), root_masks)
