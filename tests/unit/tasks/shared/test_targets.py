import pytest

from tasks.shared.targets import ALIASES, parse_platform


class TestParsePlatform:
    @pytest.mark.parametrize("alias", ALIASES)
    def test_returns_each_supported_alias_unchanged(self, alias):
        assert parse_platform(alias) == alias

    def test_rejects_an_unknown_alias(self):
        with pytest.raises(ValueError, match="unsupported platform"):
            parse_platform("windows-x64")

    def test_rejects_the_empty_string(self):
        with pytest.raises(ValueError, match="unsupported platform"):
            parse_platform("")
