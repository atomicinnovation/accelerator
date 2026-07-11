from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

import tasks.docs as td
from tasks.shared.skill_pages import DOCS_GENERATED_RELATIVE, generate_pages


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _generated_dir(root: Path) -> Path:
    return root / DOCS_GENERATED_RELATIVE


class TestGenerateCheck:
    def test_passes_when_pages_match_registered_skills(
        self, ctx, fake_repo_tree
    ):
        generate_pages(fake_repo_tree)
        td.generate_check(ctx, repo_root=str(fake_repo_tree))

    def test_missing_page_fails_naming_the_skill(self, ctx, fake_repo_tree):
        generate_pages(fake_repo_tree)
        (_generated_dir(fake_repo_tree) / "testcat/alpha.md").unlink()
        with pytest.raises(Exit, match="alpha"):
            td.generate_check(ctx, repo_root=str(fake_repo_tree))

    def test_orphan_page_fails_naming_the_page(self, ctx, fake_repo_tree):
        generate_pages(fake_repo_tree)
        orphan = _generated_dir(fake_repo_tree) / "testcat/ghost.md"
        orphan.write_text("orphan")
        with pytest.raises(Exit, match="ghost"):
            td.generate_check(ctx, repo_root=str(fake_repo_tree))

    def test_failure_names_the_fix_command(self, ctx, fake_repo_tree):
        generate_pages(fake_repo_tree)
        (_generated_dir(fake_repo_tree) / "othercat/deep/beta.md").unlink()
        with pytest.raises(Exit, match=r"mise run docs:generate"):
            td.generate_check(ctx, repo_root=str(fake_repo_tree))

    def test_missing_generated_dir_fails(self, ctx, fake_repo_tree):
        with pytest.raises(Exit, match=r"mise run docs:generate"):
            td.generate_check(ctx, repo_root=str(fake_repo_tree))

    def test_index_page_is_not_an_orphan(self, ctx, fake_repo_tree):
        generate_pages(fake_repo_tree)
        assert (_generated_dir(fake_repo_tree) / "index.md").is_file()
        td.generate_check(ctx, repo_root=str(fake_repo_tree))
