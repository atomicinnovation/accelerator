import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

import tasks.docs as td

TODAY = "2026-10-03"


def _ctx(*advisories: dict) -> MagicMock:
    vulnerabilities = {
        advisory["name"]: {"via": [advisory]} for advisory in advisories
    }
    ctx = MagicMock(spec=Context)
    ctx.run.return_value = MagicMock(
        exited=1 if advisories else 0,
        stdout=json.dumps({"vulnerabilities": vulnerabilities}),
    )
    return ctx


def _advisory(ghsa: str, severity: str = "high") -> dict:
    return {
        "name": "http-cache-semantics",
        "title": "max-stale handling can disclose cached responses",
        "url": f"https://github.com/advisories/{ghsa}",
        "severity": severity,
    }


@pytest.fixture
def ignores(tmp_path: Path):
    def write(*entries: str) -> str:
        path = tmp_path / "audit-ignores.toml"
        path.write_text("ignore = [\n" + "".join(entries) + "]\n")
        return str(path)

    return write


def _entry(ghsa: str, review_by: str = "2026-11-03") -> str:
    return (
        f'  {{ id = "{ghsa}", reason = "no fix. review-by: {review_by}" }},\n'
    )


class TestAuditCheck:
    def test_passes_with_no_advisories(self, ignores):
        td.audit_check(_ctx(), ignores=ignores(), today=TODAY)

    def test_a_high_advisory_fails_naming_it(self, ignores):
        with pytest.raises(Exit, match=r"GHSA-aaaa.*http-cache-semantics"):
            td.audit_check(
                _ctx(_advisory("GHSA-aaaa")), ignores=ignores(), today=TODAY
            )

    def test_a_moderate_advisory_passes(self, ignores):
        td.audit_check(
            _ctx(_advisory("GHSA-aaaa", "moderate")),
            ignores=ignores(),
            today=TODAY,
        )

    def test_an_ignored_advisory_passes(self, ignores):
        td.audit_check(
            _ctx(_advisory("GHSA-aaaa")),
            ignores=ignores(_entry("GHSA-aaaa")),
            today=TODAY,
        )

    def test_a_lapsed_ignore_fails_naming_it_even_with_no_advisory(
        self, ignores
    ):
        with pytest.raises(Exit, match=r"GHSA-aaaa.*2026-10-02"):
            td.audit_check(
                _ctx(),
                ignores=ignores(_entry("GHSA-aaaa", "2026-10-02")),
                today=TODAY,
            )

    def test_a_malformed_ignore_fails_naming_it(self, ignores):
        path = ignores('  { id = "GHSA-aaaa", reason = "no fix" },\n')
        with pytest.raises(Exit, match="GHSA-aaaa"):
            td.audit_check(_ctx(), ignores=path, today=TODAY)

    def test_audits_the_docs_site_as_json(self, ignores):
        ctx = _ctx()
        td.audit_check(ctx, ignores=ignores(), today=TODAY)
        command = ctx.run.call_args.args[0]
        assert "npm --prefix" in command
        assert "audit --json" in command

    def test_the_repository_ignore_list_parses(self):
        td.audit_check(_ctx(), today=TODAY)
