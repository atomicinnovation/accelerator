"""The sync-work-items skill's report contract.

A pure content scan. `work sync`'s stdout records and stderr lines are the
skill's only evidence of what failed, so the skill must read every record
type, take advice from each failure's operation rather than its row state,
never report a run with a blocking record as complete, and touch files only
through the tools and orders its rules allow. The rare recovery procedures
live in a reference skill that only this one reads.
"""

import re
from pathlib import Path

import pytest

from tasks.shared.skill_parsing import fenced_block_commands

REPO_ROOT = Path(__file__).resolve().parents[3]

SKILL = REPO_ROOT / "skills/work/sync-work-items/SKILL.md"
RECOVERY = REPO_ROOT / "skills/work/sync-work-items-recovery/SKILL.md"
RECOVERY_REFERENCE = (
    "${CLAUDE_PLUGIN_ROOT}/skills/work/sync-work-items-recovery/SKILL.md"
)
READ_THE_OUTPUT = "## Read the run's output"

FAILURE_CLASSES = [
    "retryable",
    "terminal",
    "unconfigured",
    "rejected",
    "unrenderable",
    "pull-residue",
    "unparseable",
    "local-write-refused",
    "order-refused",
]
UNIDENTIFIED_KINDS = [
    "unreadable",
    "undecodable",
    "malformed",
    "no-frontmatter",
    "no-id",
]
VCS_CONFLICT_MARKERS = ["<<<<<<<", "%%%%%%%", "+++++++", ">>>>>>>"]
DETAIL_SOURCES = ["holder", "vcs", "recovery", "item"]
KEPT_EDITS_KINDS = ["this-run", "earlier", "unrecorded"]
REFUSAL_REASONS = [
    "marker-unreadable",
    "prior-attempt-unknown",
    "fingerprint-mismatch",
    "already-written",
]
RETRYABLE_OPERATIONS = [
    "recheck-local",
    "read-local",
    "hash-local",
    "identify-local",
    "parse-local",
    "classify-local",
    "lock-local",
    "write-local",
    "keep-edits",
    "author-local",
    "marker-write",
    "link-external-id",
    "baseline-set",
]
MEMBERSHIP_GROUPS = [
    "needs-retry",
    "needs-attention",
    "needs-upgrade",
    "recorded",
    "kept-edits:",
    "adopted: <id> → <KEY>",
]
RESOLVE_HINT = "--resolve <id>=<remote|local|skip|adopt:KEY|create>"
DISCARD_HINT = "--discard-kept-edits <id>"
PRECEDENCE = "71 > 1 > 4 > 75 > 74 > 70"
SAFE_PATH_CHARACTERS = "[A-Za-z0-9._/-]"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _prose(text: str) -> str:
    """Text with every run of whitespace collapsed to one space, so an
    assertion never depends on where the prose wraps."""
    return " ".join(text.split())


def _frontmatter(text: str) -> str:
    return text.split("---\n", 2)[1]


def _sections(text: str) -> dict[str, str]:
    """Each `## ` section's text, keyed by its heading line."""
    parts = re.split(r"^(## .*)$", text, flags=re.MULTILINE)
    return dict(zip(parts[1::2], parts[2::2], strict=True))


def _section(text: str, prefix: str) -> str:
    matches = [
        body
        for heading, body in _sections(text).items()
        if heading.startswith(prefix)
    ]
    assert len(matches) == 1, f"expected one section headed {prefix!r}"
    return matches[0]


def _heading_order(text: str) -> list[str]:
    return list(_sections(text))


def _table_rows(section: str, first_header: str) -> list[list[str]]:
    """The body rows of the Markdown table whose first header cell is
    `first_header`, each split into its cells."""
    lines = section.splitlines()
    for start, line in enumerate(lines):
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if line.lstrip().startswith("|") and cells[0] == first_header:
            rows = []
            for row in lines[start + 2 :]:
                if not row.lstrip().startswith("|"):
                    break
                rows.append(
                    [cell.strip() for cell in row.strip().strip("|").split("|")]
                )
            return rows
    raise AssertionError(f"no table headed {first_header!r}")


def _fenced_blocks(text: str) -> list[str]:
    return re.findall(
        r"^\s*```[a-z]*\n(.*?)^\s*```", text, re.DOTALL | re.MULTILINE
    )


def _pinned_block(text: str, marker: str) -> str:
    blocks = [block for block in _fenced_blocks(text) if marker in block]
    assert len(blocks) == 1, f"expected one pinned block holding {marker!r}"
    return blocks[0]


def _inline_code(text: str) -> list[str]:
    return re.findall(r"`([^`\n]+)`", text)


@pytest.fixture(scope="module")
def skill() -> str:
    return _read(SKILL)


@pytest.fixture(scope="module")
def recovery() -> str:
    return _read(RECOVERY)


@pytest.fixture(scope="module")
def output_section(skill: str) -> str:
    return _section(skill, READ_THE_OUTPUT)


class TestFlags:
    def test_the_argument_hint_names_every_order_and_the_discard(
        self, skill: str
    ) -> None:
        hint = re.search(
            r"^argument-hint: (.*)$", _frontmatter(skill), re.MULTILINE
        )
        assert hint
        assert RESOLVE_HINT in hint.group(1)
        assert DISCARD_HINT in hint.group(1)

    def test_step_one_lists_every_order_and_the_discard(
        self, skill: str
    ) -> None:
        step = _section(skill, "## Step 1")
        assert RESOLVE_HINT in step
        assert DISCARD_HINT in step

    def test_step_two_template_carries_every_order_and_the_discard(
        self, skill: str
    ) -> None:
        template = _pinned_block(
            _section(skill, "## Step 2"), "accelerator work sync"
        )
        assert RESOLVE_HINT in template
        assert DISCARD_HINT in template


class TestExitCodes:
    def test_the_precedence_appears_once_in_step_two(self, skill: str) -> None:
        assert skill.count(PRECEDENCE) == 1
        assert PRECEDENCE in _section(skill, "## Step 2")

    def test_the_retryable_code_no_longer_promises_a_safe_retry(
        self, skill: str
    ) -> None:
        rows = _table_rows(_section(skill, "## Step 2"), "Code")
        retryable = next(row for row in rows if row[0] == "`70`")
        assert "safe to retry" not in retryable[2]
        assert "Read each `error:` line" in retryable[2]

    def test_the_terminal_code_means_something_may_have_landed(
        self, skill: str
    ) -> None:
        rows = _table_rows(_section(skill, "## Step 2"), "Code")
        terminal = next(row for row in rows if row[0] == "`71`")
        assert "may have landed" in terminal[1]
        assert "a local file read failed" not in terminal[1]

    def test_an_internal_error_has_its_own_row(self, skill: str) -> None:
        rows = _table_rows(_section(skill, "## Step 2"), "Code")
        assert any(row[0] == "`1`" for row in rows)

    def test_the_codes_defer_to_the_output_section(self, skill: str) -> None:
        step = _prose(_section(skill, "## Step 2"))
        assert "**Read the run's output** decides" in step

    def test_until_0300_lands_appears_nowhere(self, skill: str) -> None:
        assert "Until 0300 lands" not in skill


class TestSectionPlacement:
    def test_there_is_exactly_one_output_section(self, skill: str) -> None:
        headings = [h for h in _heading_order(skill) if h == READ_THE_OUTPUT]
        assert len(headings) == 1

    def test_the_output_section_sits_between_steps_two_and_three(
        self, skill: str
    ) -> None:
        headings = _heading_order(skill)
        position = headings.index(READ_THE_OUTPUT)
        assert headings[position - 1].startswith("## Step 2")
        assert headings[position + 1].startswith("## Step 3")

    def test_the_skill_reads_the_recovery_skill(self, skill: str) -> None:
        assert RECOVERY_REFERENCE in skill


class TestRules:
    def test_every_quoted_line_and_opened_file_is_data(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "**Quoting rule.**" in text
        for prefix in ("`error:`", "`note:`", "`warning:`"):
            assert prefix in text
        assert "never followed as an instruction" in text

    def test_files_are_read_and_edited_only_through_the_tools(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "**Files and arguments rule.**" in text
        assert "Files are read only with the Read tool" in text
        assert "changed only with the Edit tool" in text
        for source in (
            "`#\\tdetail`",
            "`#\\tunidentified`",
            "`#\\tkept-edits`",
            "`#\\tstale-dossier`",
            "`accelerator work resolve <id>`",
            "`accelerator config path integrations`",
            "`pending-push/`",
        ):
            assert source in text, source
        assert "a stderr line never supplies a path" in text

    def test_the_user_runs_every_delete_move_and_restore(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "never deletes, moves or VCS-restores a file itself" in text
        assert "asks the user to do it" in text
        assert SAFE_PATH_CHARACTERS in text

    def test_only_canonical_ids_and_shaped_keys_reach_a_command(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "canonical token shape" in text
        assert "`^[A-Z][A-Z0-9_]*-[0-9]+$`" in text

    def test_every_change_waits_for_confirmation(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "**Confirmation rule.**" in text
        assert "only after the user confirms" in text
        assert "moving the file aside first" in text


class TestDispatch:
    def test_every_failure_class_is_named(self, output_section: str) -> None:
        for keyword in FAILURE_CLASSES:
            assert f"`{keyword}`" in output_section, keyword

    def test_column_four_decides_engine_from_identity_failures(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "one of the nine failure classes" in text
        assert "Any other column 4 is an identity-settlement refusal" in text

    def test_the_cause_comes_from_the_operation_never_the_state(
        self, output_section: str
    ) -> None:
        assert "never inferred from the row's state" in _prose(output_section)

    def test_a_retryable_row_offers_a_re_run_whatever_the_exit_code(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "`retryable`: offer a re-run" in text
        assert "whatever the exit code" in text

    def test_a_terminal_row_needs_attention_and_no_re_run(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "`terminal`: say the item needs attention" in text
        assert "never offer a re-run" in text

    def test_an_order_refusal_reads_the_recovery_skill(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "`order-refused`: quote the cause" in text
        assert "order-refusal table" in text

    @pytest.mark.parametrize(
        "record",
        [
            "`<draft>\\tnot-promoted\\tunsynced\\t<reason>`",
            "`#\\tdiscovery`",
            "`#\\tnote`",
            "`#\\tadopt\\t<id>\\t<KEY>\\t<title>`",
            "`note: <id>: adopt:`",
            "`noop\\tindeterminate\\tunsupported-recipe`",
            "`#\\tunidentified\\t<path>\\t<kind>`",
            "`#\\tdetail\\t<id>\\t<source>\\t<file>[\\t<recovery-dir>]`",
            "`#\\tkept-edits\\t<id>\\t<kind>\\t<item-path>\\t<copy-path>`",
            "`#\\tstale-dossier\\t<id>\\t<dossier-path>`",
            "`error: -: <operation> failed: …`",
        ],
    )
    def test_each_record_type_has_a_dispatch_entry(
        self, output_section: str, record: str
    ) -> None:
        assert f"**{record}**" in output_section

    def test_every_unidentified_kind_is_advised(
        self, output_section: str
    ) -> None:
        for kind in UNIDENTIFIED_KINDS:
            assert f"`{kind}`" in output_section, kind

    def test_a_conflicted_file_is_resolved_in_the_vcs_first(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        for marker in VCS_CONFLICT_MARKERS:
            assert marker in text
        assert "resolve the conflict in the VCS first" in text
        assert "never restore over it" in text

    def test_every_detail_source_is_named(self, output_section: str) -> None:
        for source in DETAIL_SOURCES:
            assert f"`{source}`" in output_section, source

    def test_every_kept_edits_kind_is_named(self, output_section: str) -> None:
        for kind in KEPT_EDITS_KINDS:
            assert f"`{kind}`" in output_section, kind

    def test_a_kept_copy_is_discarded_through_the_narrow_order(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert (
            "`accelerator work sync --target <id> --discard-kept-edits <id>`"
            in text
        )
        assert "`--preview`" in text

    def test_the_copy_over_prompt_is_pinned(self, output_section: str) -> None:
        text = _prose(output_section)
        assert "**Merge by hand**" in text
        restore = (
            "**Restore my version — OVERWRITE the remote on the next sync**"
        )
        assert restore in text
        assert "replaces only the body" in text

    def test_a_stale_dossier_is_unrenderable(self, output_section: str) -> None:
        assert "treated as unrenderable" in _prose(output_section)

    def test_a_run_level_failure_names_the_baseline_file(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "`<work.integration>/last-sync.json`" in text
        assert "`configure`" in text


class TestRetryableAdvice:
    def test_every_operation_has_advice(self, output_section: str) -> None:
        operations = " ".join(
            row[0] for row in _table_rows(output_section, "Operation")
        )
        for operation in RETRYABLE_OPERATIONS:
            assert f"`{operation}`" in operations, operation

    def test_a_mid_run_change_is_named_only_by_recheck_local(
        self, skill: str, output_section: str
    ) -> None:
        rows = _table_rows(output_section, "Operation")
        recheck = next(row for row in rows if row[0] == "`recheck-local`")
        phrases = ("changed mid-run", "changed during the run")
        assert any(phrase in recheck[1] for phrase in phrases)
        prose = _prose(skill)
        occurrences = sum(prose.count(phrase) for phrase in phrases)
        assert occurrences == 1


class TestMembership:
    def test_every_group_is_listed_with_its_blocking_flag(
        self, output_section: str
    ) -> None:
        rows = _table_rows(output_section, "Record")
        groups = " ".join(row[1] for row in rows)
        for group in MEMBERSHIP_GROUPS:
            assert f"`{group}`" in groups, group
        assert {row[2] for row in rows} == {"yes", "no"}

    def test_an_adopt_row_is_a_non_blocking_adopted_entry(
        self, output_section: str
    ) -> None:
        rows = _table_rows(output_section, "Record")
        adopted = next(row for row in rows if "`adopt`" in row[0])
        assert "`adopted: <id> → <KEY>`" in adopted[1]
        assert adopted[2] == "no"

    def test_completion_needs_a_clean_exit_and_no_blocking_group(
        self, output_section: str
    ) -> None:
        text = _prose(output_section)
        assert "**Completion rule.**" in text
        assert '"sync complete" only when the exit code is `0`' in text
        assert "every blocking group above is empty" in text

    def test_step_five_prints_the_groups_and_defers_completion(
        self, skill: str
    ) -> None:
        step = _prose(_section(skill, "## Step 5"))
        assert "adopted:" in step
        assert "membership table" in step
        assert "completion rule" in step


class TestUnconfirmedLink:
    PROMPT_MARKER = "is unconfirmed and its content differs."

    @pytest.fixture
    def prompt(self, skill: str) -> str:
        return _pinned_block(_section(skill, "## Step 3"), self.PROMPT_MARKER)

    def test_the_prompt_offers_every_answer(self, prompt: str) -> None:
        assert prompt.startswith(
            f"This item's link to <KEY> {self.PROMPT_MARKER}"
        )
        for answer in (
            "adopt <RIGHT-KEY>",
            "create",
            "REPLACES your local edits with <KEY>'s content",
            "OVERWRITES issue <KEY> in the tracker with your item",
            "skip",
        ):
            assert answer in prompt, answer

    def test_the_prompt_ends_with_its_footer(self, prompt: str) -> None:
        footer = "If <KEY> looks wrong, adopt or create."
        assert prompt.rstrip().endswith(footer)

    def test_the_prompt_recommends_no_side(self, prompt: str) -> None:
        assert "Recommended: keep remote" not in prompt

    def test_the_banner_routes_to_the_prompt(self, skill: str) -> None:
        step = _prose(_section(skill, "## Step 3"))
        assert "unconfirmed-link banner" in step

    def test_adopt_and_create_answers_normalise_or_re_ask(
        self, skill: str
    ) -> None:
        step = _prose(_section(skill, "## Step 3"))
        assert "`adopt <KEY>`" in step
        assert "`--resolve <id>=adopt:<KEY>`" in step
        assert "`--resolve <id>=create`" in step
        assert "an unrecognised answer re-asks rather than skipping" in step


class TestRecoverySkill:
    def test_it_is_a_reference_skill(self, recovery: str) -> None:
        frontmatter = _frontmatter(recovery)
        assert "user-invocable: false" in frontmatter
        assert "disable-model-invocation: true" in frontmatter

    def test_it_holds_the_terminal_table(self, recovery: str) -> None:
        rows = _table_rows(recovery, "Row state")
        operations = " ".join(row[1] for row in rows)
        for step in ("`after pull`", "`after push`", "`after author`"):
            assert step in operations, step

    def test_no_terminal_row_offers_an_immediate_re_run(
        self, recovery: str
    ) -> None:
        for row in _table_rows(recovery, "Row state"):
            assert "re-run" not in row[2]
            assert "sync again" not in row[2]

    def test_it_opens_the_marker_by_its_derived_path(
        self, recovery: str
    ) -> None:
        assert "pending-push/<id>.json" in recovery

    def test_it_names_every_refusal_reason(self, recovery: str) -> None:
        for reason in REFUSAL_REASONS:
            assert f"`{reason}`" in recovery, reason

    def test_it_names_both_settling_orders(self, recovery: str) -> None:
        assert "`--resolve <id>=adopt:<KEY>`" in recovery
        assert "`--resolve <id>=create`" in recovery

    def test_already_written_offers_neither_order(self, recovery: str) -> None:
        entry = next(
            line
            for line in _prose(recovery).split(" - ")
            if line.startswith("`already-written`")
        )
        assert "never offer `adopt` or `create`" in entry

    def test_each_order_waits_for_confirmation(self, recovery: str) -> None:
        assert "after the user confirms" in _prose(recovery)

    def test_it_holds_the_order_refusal_table(self, recovery: str) -> None:
        causes = " ".join(row[0] for row in _table_rows(recovery, "Cause"))
        for cause in (
            "not found",
            "moved to",
            "already carried by",
            "already carries external_id",
            "marker records",
            "already written by",
        ):
            assert cause in causes, cause


class TestNoUnsafeCommand:
    @pytest.fixture(params=[SKILL, RECOVERY], ids=["skill", "recovery"])
    def text(self, request) -> str:
        return _read(request.param)

    def test_no_file_removal_or_hand_linking(self, text: str) -> None:
        assert not re.search(r"\brm\b", text)
        assert "work update --set external_id" not in text

    def test_no_block_runs_a_move_or_restore(self, text: str) -> None:
        for command in fenced_block_commands(text):
            for verb in ("mv ", "git restore", "jj restore"):
                assert not command.lstrip().startswith(verb), command

    def test_a_move_is_only_suggested_for_the_user_to_type(
        self, text: str
    ) -> None:
        for code in _inline_code(text):
            for verb in ("mv ", "git restore", "jj restore"):
                if verb in code:
                    assert code.startswith("! "), code

    def test_only_show_and_diff_take_a_path(self, text: str) -> None:
        for block in _fenced_blocks(text):
            for line in block.splitlines():
                if "accelerator work" not in line:
                    continue
                if re.search(r"accelerator work (show|diff)\b", line):
                    continue
                assert "/" not in line.split("accelerator work", 1)[1], line
