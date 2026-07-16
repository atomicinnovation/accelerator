import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.docs as td
from tasks.shared.skill_pages import (
    DOCS_GENERATED_RELATIVE,
    SkillPage,
    SkillPageError,
    discover_skills,
    generate_pages,
    output_path,
    render_index,
    sanitise_body,
)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _generated_dir(root: Path) -> Path:
    return root / DOCS_GENERATED_RELATIVE


# ── discovery ─────────────────────────────────────────────────────────


class TestDiscovery:
    def test_finds_registered_skills(self, fake_repo_tree):
        names = {p.name for p in discover_skills(fake_repo_tree)}
        assert names == {"alpha", "hidden", "beta"}

    def test_excludes_node_modules_and_test_fixtures(self, fake_repo_tree):
        sources = [
            p.source.relative_to(fake_repo_tree).as_posix()
            for p in discover_skills(fake_repo_tree)
        ]
        assert not any("node_modules" in s for s in sources)
        assert not any("test-fixtures" in s for s in sources)

    def test_excludes_unregistered_skills(self, fake_repo_tree):
        names = {p.name for p in discover_skills(fake_repo_tree)}
        assert "gamma" not in names

    def test_category_is_relative_parent_within_skills(self, fake_repo_tree):
        by_name = {p.name: p for p in discover_skills(fake_repo_tree)}
        assert by_name["alpha"].category == "testcat"
        assert by_name["beta"].category == "othercat/deep"

    def test_non_mapping_frontmatter_names_the_file(self, fake_repo_tree):
        bad = fake_repo_tree / "skills/testcat/bad/SKILL.md"
        bad.parent.mkdir()
        bad.write_text("---\njust a scalar\n---\n\nX.\n")
        with pytest.raises(SkillPageError, match=r"not a mapping in .*bad"):
            discover_skills(fake_repo_tree)

    def test_missing_name_names_the_file(self, fake_repo_tree):
        bad = fake_repo_tree / "skills/testcat/bad/SKILL.md"
        bad.parent.mkdir()
        bad.write_text("---\ndescription: No name here.\n---\n\nX.\n")
        with pytest.raises(SkillPageError, match=r"no 'name' in .*bad"):
            discover_skills(fake_repo_tree)

    def test_skill_outside_category_directory_names_the_file(
        self, fake_repo_tree
    ):
        plugin = fake_repo_tree / ".claude-plugin/plugin.json"
        data = json.loads(plugin.read_text())
        data["skills"].append("./skills/shallow/")
        plugin.write_text(json.dumps(data))
        bad = fake_repo_tree / "skills/shallow/SKILL.md"
        bad.parent.mkdir()
        bad.write_text("---\nname: shallow\ndescription: X.\n---\n\nX.\n")
        with pytest.raises(
            SkillPageError, match=r"not under a category directory: .*shallow"
        ):
            discover_skills(fake_repo_tree)

    def test_folded_multiline_description_parses(self, fake_repo_tree):
        by_name = {p.name: p for p in discover_skills(fake_repo_tree)}
        assert by_name["alpha"].description == (
            "Create things interactively. Use when the user wants to "
            "create a thing through iterative collaboration."
        )


# ── sanitisation ──────────────────────────────────────────────────────


class TestSanitisation:
    def test_whole_line_preprocessor_neutralised(self):
        out = sanitise_body("!`scripts/status.sh`\n")
        assert out == "`!scripts/status.sh`\n"

    def test_inline_preprocessor_neutralised(self):
        out = sanitise_body("**Dir**: !`read-path.sh plans`\n")
        assert out == "**Dir**: `!read-path.sh plans`\n"

    def test_angle_brackets_escaped_outside_fences(self):
        out = sanitise_body("Use <ID> here.\n")
        assert out == "Use \\<ID> here.\n"

    def test_angle_brackets_untouched_inside_fences(self):
        body = "```\n<ID> and !`cmd`\n```\n"
        assert sanitise_body(body) == body

    def test_angle_brackets_untouched_inside_inline_code(self):
        out = sanitise_body("Keep `<KEPT>` but escape <GONE>.\n")
        assert out == "Keep `<KEPT>` but escape \\<GONE>.\n"

    def test_tilde_fences_respected(self):
        body = "~~~\n<ID>\n~~~\n"
        assert sanitise_body(body) == body


# ── generation ────────────────────────────────────────────────────────


class TestGeneration:
    def test_writes_one_page_per_skill_plus_index(self, fake_repo_tree):
        written = generate_pages(fake_repo_tree)
        gen = _generated_dir(fake_repo_tree)
        assert len(written) == 3
        assert (gen / "testcat/alpha.md").is_file()
        assert (gen / "othercat/deep/beta.md").is_file()
        assert (gen / "index.md").is_file()

    def test_internal_skill_routed_under_internal_with_badge(
        self, fake_repo_tree
    ):
        generate_pages(fake_repo_tree)
        gen = _generated_dir(fake_repo_tree)
        page = (gen / "internal/hidden.md").read_text()
        assert not (gen / "testcat/hidden.md").exists()
        assert "text: Internal" in page
        assert "variant: caution" in page

    def test_page_has_starlight_frontmatter_and_header(self, fake_repo_tree):
        generate_pages(fake_repo_tree)
        page = (_generated_dir(fake_repo_tree) / "testcat/alpha.md").read_text()
        assert page.startswith("---\n")
        assert "title: alpha" in page
        assert "label: alpha" in page
        assert "**Argument hint:** `[optional thing]`" in page
        assert "**Allowed tools:**" in page
        assert "${CLAUDE_PLUGIN_ROOT}/scripts/*" in page

    def test_page_body_is_sanitised(self, fake_repo_tree):
        generate_pages(fake_repo_tree)
        page = (_generated_dir(fake_repo_tree) / "testcat/alpha.md").read_text()
        assert "!`${CLAUDE_PLUGIN_ROOT}/scripts/status.sh`" not in page
        assert "`!${CLAUDE_PLUGIN_ROOT}/scripts/status.sh`" in page
        assert "`!${CLAUDE_PLUGIN_ROOT}/scripts/read-path.sh plans`" in page
        assert "\\<ID>" in page
        assert "0001-<literal>-example" in page
        assert "!`${CLAUDE_PLUGIN_ROOT}/scripts/fenced.sh`" in page

    def test_invocability_rendered(self, fake_repo_tree):
        generate_pages(fake_repo_tree)
        gen = _generated_dir(fake_repo_tree)
        assert (
            "User- and model-invocable"
            in (gen / "testcat/alpha.md").read_text()
        )
        assert (
            "User-invoked only" in (gen / "othercat/deep/beta.md").read_text()
        )
        assert "not user-invocable" in (gen / "internal/hidden.md").read_text()

    def test_index_lists_every_skill(self, fake_repo_tree):
        generate_pages(fake_repo_tree)
        index = (_generated_dir(fake_repo_tree) / "index.md").read_text()
        assert "[alpha](testcat/alpha.md)" in index
        assert "[beta](othercat/deep/beta.md)" in index
        assert "[hidden](internal/hidden.md)" in index
        assert "## Internal" in index
        assert index.count("[hidden]") == 1
        assert "[hidden](testcat/hidden.md)" not in index

    def test_index_gloss_is_first_sentence_only(self, fake_repo_tree):
        generate_pages(fake_repo_tree)
        index = (_generated_dir(fake_repo_tree) / "index.md").read_text()
        assert (
            "- [alpha](testcat/alpha.md) — Create things interactively.\n"
            in index
        )

    def test_index_gloss_survives_abbreviations(self, fake_repo_tree):
        page = SkillPage(
            name="abbrev",
            category="testcat",
            source=fake_repo_tree / "skills/testcat/abbrev/SKILL.md",
            frontmatter={
                "name": "abbrev",
                "description": (
                    "Look up keys (e.g. PROJ-123, i.e. issue keys) in the "
                    "tracker. Second sentence."
                ),
            },
            body="X.\n",
        )
        index = render_index([page])
        assert (
            "— Look up keys (e.g. PROJ-123, i.e. issue keys) in the tracker."
            in index
        )
        assert "Second sentence" not in index

    def test_regeneration_removes_stale_pages(self, fake_repo_tree):
        gen = _generated_dir(fake_repo_tree)
        stale = gen / "testcat/renamed-away.md"
        stale.parent.mkdir(parents=True)
        stale.write_text("stale")
        generate_pages(fake_repo_tree)
        assert not stale.exists()
        assert (gen / "testcat/alpha.md").is_file()

    def test_output_paths_stable_across_runs(self, fake_repo_tree):
        first = generate_pages(fake_repo_tree)
        second = generate_pages(fake_repo_tree)
        assert first == second

    def test_output_path_collision_raises(self, fake_repo_tree):
        clash = fake_repo_tree / "skills/testcat/alpha2/SKILL.md"
        clash.parent.mkdir()
        clash.write_text("---\nname: alpha\ndescription: Dup.\n---\n\nX.\n")
        with pytest.raises(
            SkillPageError,
            match=r"collision.*alpha/SKILL\.md.*alpha2/SKILL\.md",
        ):
            generate_pages(fake_repo_tree)

    def test_output_path_uses_skill_name(self, fake_repo_tree):
        gen = _generated_dir(fake_repo_tree)
        by_name = {p.name: p for p in discover_skills(fake_repo_tree)}
        assert output_path(by_name["alpha"], gen) == (gen / "testcat/alpha.md")
        assert output_path(by_name["hidden"], gen) == (
            gen / "internal/hidden.md"
        )


# ── invoke task ───────────────────────────────────────────────────────


class TestGenerateTask:
    def test_task_generates_with_injected_root(self, ctx, fake_repo_tree):
        td.generate(ctx, repo_root=str(fake_repo_tree))
        assert (_generated_dir(fake_repo_tree) / "index.md").is_file()
