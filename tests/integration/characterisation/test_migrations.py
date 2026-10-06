import json
import os

import pytest

from tests.integration.support.characterisation import REVISION, Mask

MIGRATIONS = (
    "0001-rename-tickets-to-work",
    "0002-rename-work-items-with-project-prefix",
    "0003-relocate-accelerator-state",
    "0004-restructure-meta-research-into-subject-subcategories",
    "0005-rename-work-item-type-to-kind",
    "0006-canonicalise-work-item-id-and-author",
    "0007-unify-meta-corpus-frontmatter",
    "0008-canonical-frontmatter-quoting",
    "0009-split-work-key-from-tracker-scope-key",
    "0010-strip-research-title-prefix",
)
APPLIED = ".accelerator/state/migrations-applied"
RUN_LOCK = ".accelerator/state/migrate-run.lockdir"
SYNCED_DIGEST = (
    "c957188894f0f526ea343d22655e28798d995b35a1a04958f4ca2c7db42def19"
)
MODIFIED_DIGEST = "0" * 64

_OBSERVED_ROOTS = ("meta", ".claude", ".accelerator")


def _pending_only(repository, migration):
    applied = [m for m in MIGRATIONS if m != migration]
    repository.write(APPLIED, "".join(f"{m}\n" for m in applied))


def _migrate(run, binaries, repository, *args):
    revisions = repository.revisions()
    result = run(binaries.subbinary("migrate"), *args, cwd=repository.root)
    return result, [*revisions, REVISION]


def _observe_tree(result, root, *, under=_OBSERVED_ROOTS):
    for top in under:
        base = root / top
        if not base.exists():
            continue
        for path in sorted(base.rglob("*")):
            relative = path.relative_to(root)
            if path.is_dir():
                continue
            result = result.with_observation(
                f"{relative}:\n{path.read_text().rstrip()}"
            )
    return result


_TICKET = "---\nticket_id: 0001\n---\n\n# 0001: Foo\n"
_REVIEW = "---\ntype: work-item-review\n---\n\n# foo-review-1\n"

_LEGACY_CONFIGS = {
    "valid": "---\npaths:\n  tickets: meta/tickets\n---\n",
    "no-frontmatter": "paths are defaults\n",
    "empty": "---\n---\n",
    "null-root": "---\nnull\n---\n",
    "sequence-root": "---\n- a\n- b\n---\n",
    "scalar-root": "---\njust text\n---\n",
    "unterminated-fence": "---\npaths:\n  tickets: meta/tickets\n",
    "invalid-yaml": "---\npaths: [unclosed\n---\n",
}


@pytest.mark.parametrize("legacy", sorted(_LEGACY_CONFIGS))
def test_m0001_over_a_legacy_config(
    repository, binaries, run, matches_golden, legacy
):
    repository.write("meta/tickets/0001-foo.md", _TICKET)
    repository.write("meta/reviews/tickets/foo-review-1.md", _REVIEW)
    repository.write(".claude/accelerator.md", _LEGACY_CONFIGS[legacy])
    _pending_only(repository, MIGRATIONS[0])
    repository.commit()

    result, masks = _migrate(run, binaries, repository)

    matches_golden(_observe_tree(result, repository.root), masks)


_PROJECT_PATTERN = (
    '---\nwork:\n  id_pattern: "{project}-{number:04d}"\n'
    "  default_project_code: ENG\n---\n"
)
_PROJECT_PATTERN_WITHOUT_CODE = (
    '---\nwork:\n  id_pattern: "{project}-{number:04d}"\n---\n'
)


@pytest.mark.parametrize(
    "config",
    [_PROJECT_PATTERN, _PROJECT_PATTERN_WITHOUT_CODE],
    ids=["with-project-code", "without-project-code"],
)
def test_m0002_renames_work_items_to_the_project_pattern(
    repository, binaries, run, matches_golden, config
):
    repository.write(".accelerator/config.md", config)
    repository.write("meta/work/0001-foo.md", _work_item("0001", "Foo"))
    repository.write(
        "meta/work/0002-bar.md",
        _work_item("0002", "Bar", 'parent: "0001"\n'),
    )
    repository.write(
        "meta/plans/2026-01-01-0001-foo.md",
        '---\ntype: plan\nwork_item_id: "0001"\n---\n\n'
        "# Plan\n\nImplements 0001.\n",
    )
    _pending_only(repository, MIGRATIONS[1])
    repository.commit()

    result, masks = _migrate(run, binaries, repository)

    matches_golden(_observe_tree(result, repository.root), masks)


def _work_item(number, title, extra=""):
    return (
        f'---\ntype: work-item\nid: "{number}"\ntitle: {title}\n'
        'date: "2026-01-01T00:00:00Z"\nauthor: a\ntags: []\nkind: task\n'
        f"status: draft\npriority: medium\n{extra}"
        'last_updated: "2026-01-01T00:00:00Z"\nlast_updated_by: a\n'
        f"schema_version: 1\n---\n\n# {number}: {title}\nBody\n"
    )


def _baseline(integrations):
    integrations.mkdir(parents=True, exist_ok=True)
    baseline = {
        "timestamp": 1,
        "items": {
            "0001": {
                "remote_updated_at": "2026-06-01T00:00:00Z",
                "remote_hash": "rh",
                "local_hash": SYNCED_DIGEST,
            },
            "0002": {
                "remote_updated_at": "2026-06-01T00:00:00Z",
                "remote_hash": "rh",
                "local_hash": MODIFIED_DIGEST,
            },
        },
    }
    (integrations / "jira").mkdir(exist_ok=True)
    (integrations / "jira/last-sync.json").write_text(
        json.dumps(baseline) + "\n"
    )


@pytest.mark.parametrize("integrations", ["relative", "absolute"])
def test_m0008_re_renders_and_realigns_sync_baselines(
    repository, binaries, run, matches_golden, fixture_root, integrations
):
    repository.write(
        "meta/work/0001-synced-item.md",
        _work_item("0001", "Synced item", "external_id: ENG-1\n"),
    )
    repository.write(
        "meta/work/0002-modified.md",
        _work_item("0002", "Modified", "external_id: ENG-2\n"),
    )
    repository.write(
        "meta/work/0003-commented.md",
        _work_item("0003", "Commented", "# a frontmatter comment\n"),
    )
    if integrations == "absolute":
        root = fixture_root / "outside-integrations"
        repository.write(
            ".accelerator/config.md",
            f"---\npaths:\n  integrations: {root}\n---\n",
        )
    else:
        root = repository.root / ".accelerator/state/integrations"
    _baseline(root)
    _pending_only(repository, MIGRATIONS[7])
    repository.commit()

    result, masks = _migrate(run, binaries, repository)

    result = result.with_observation(
        f"jira/last-sync.json:\n{(root / 'jira/last-sync.json').read_text()}"
    )
    matches_golden(
        _observe_tree(result, repository.root, under=("meta",)),
        [
            Mask(SYNCED_DIGEST, "SYNCED_DIGEST"),
            Mask(MODIFIED_DIGEST, "MODIFIED_DIGEST"),
            *masks,
        ],
    )


_UNRENDERABLE = {
    "no-frontmatter": "plain prose\n",
    "sequence-root": "---\n- a\n- b\n---\nbody\n",
    "scalar-root": "---\njust text\n---\nbody\n",
}


@pytest.mark.parametrize("content", sorted(_UNRENDERABLE))
def test_m0008_over_a_file_without_a_mapping_root(
    repository, binaries, run, matches_golden, content
):
    repository.write("meta/plans/odd.md", _UNRENDERABLE[content])
    _pending_only(repository, MIGRATIONS[7])
    repository.commit()

    result, masks = _migrate(run, binaries, repository)

    matches_golden(
        _observe_tree(result, repository.root, under=("meta",)), masks
    )


def test_a_live_run_lock_refuses_a_second_run(
    repository, binaries, run, matches_golden
):
    repository.write("README.md", "fixture\n")
    _pending_only(repository, MIGRATIONS[7])
    repository.commit()
    lock = repository.root / RUN_LOCK
    lock.mkdir(parents=True)
    (lock / "owner.0123456789abcdef").write_text(str(os.getpid()))

    result, masks = _migrate(run, binaries, repository)

    matches_golden(result, [Mask(str(os.getpid()), "LIVE_PID"), *masks])


_TYPED_TICKET = (
    "---\nticket_id: 0001\ntitle: Foo\ntype: task\nstatus: draft\n"
    "priority: medium\ndate: 2026-01-01\nauthor: a\n---\n\n# 0001: Foo\n"
)


@pytest.mark.parametrize(
    "ticket",
    [_TICKET, _TYPED_TICKET],
    ids=["untyped-ticket-with-review", "typed-ticket"],
)
def test_the_full_registry_over_an_old_repository(
    repository, binaries, run, matches_golden, ticket
):
    repository.write("meta/tickets/0001-foo.md", ticket)
    if ticket is _TICKET:
        repository.write("meta/reviews/tickets/foo-review-1.md", _REVIEW)
    repository.write(".claude/accelerator.md", _LEGACY_CONFIGS["valid"])
    repository.commit()

    result, masks = _migrate(run, binaries, repository)

    matches_golden(_observe_tree(result, repository.root), masks)
