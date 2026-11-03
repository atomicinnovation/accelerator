from tasks.shared.sources import repo_root

CONTRIBUTING = repo_root() / "CONTRIBUTING.md"
HEADING = "## Faster local builds on macOS"


def _host_settings_prose() -> str:
    text = CONTRIBUTING.read_text()
    start = text.index(HEADING)
    end = text.find("\n## ", start + len(HEADING))
    return " ".join(text[start : end if end != -1 else None].split())


class TestHostSettingsGuidance:
    def test_it_follows_local_development(self):
        text = CONTRIBUTING.read_text()
        assert text.index("## Local development") < text.index(HEADING)

    def test_it_gives_the_developer_tools_steps(self):
        section = _host_settings_prose()
        for required in ("Developer Tools", "Restart", "tmux", "zellij"):
            assert required in section, required

    def test_it_names_the_folders_to_exclude(self):
        section = _host_settings_prose()
        for required in (
            "`cli/target`",
            "`cli/visualiser/frontend/node_modules`",
            "`docs-site/node_modules`",
            "Excluded",
        ):
            assert required in section, required

    def test_it_states_the_exemptions_security_tradeoff(self):
        assert (
            "XProtect no longer scans anything an exempted app launches"
            in _host_settings_prose()
        )

    def test_it_states_that_an_exclusion_does_not_stop_file_watching(self):
        assert (
            "An exclusion stops indexing, not file watching"
            in _host_settings_prose()
        )
