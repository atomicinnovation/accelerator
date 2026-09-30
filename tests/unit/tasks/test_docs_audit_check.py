import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

import tasks.docs as td

_ADVISORY = "GHSA-ch52-4w7c-c8xp"


def _advisory(ghsa: str, severity: str) -> dict:
    return {
        "source": 1,
        "url": f"https://github.com/advisories/{ghsa}",
        "severity": severity,
        "title": f"advisory {ghsa}",
    }


def _report(*vulnerabilities: tuple[str, str, list]) -> str:
    return json.dumps(
        {
            "auditReportVersion": 2,
            "vulnerabilities": {
                name: {"name": name, "severity": severity, "via": via}
                for name, severity, via in vulnerabilities
            },
        }
    )


_CLEAN_REPORT = _report()
_HIGH_REPORT = _report(
    ("http-cache-semantics", "high", [_advisory(_ADVISORY, "high")]),
    ("astro", "high", ["http-cache-semantics"]),
)


def _ctx(audit_stdout: str) -> MagicMock:
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(exited=1, stdout=audit_stdout)
    return m


def _exceptions(tmp_path: Path, *ids: str) -> str:
    path = tmp_path / "audit-exceptions.toml"
    path.write_text(
        "".join(
            f'[[exception]]\nid = "{i}"\nreason = "no fix upstream"\n\n'
            for i in ids
        )
    )
    return str(path)


class TestAuditCheck:
    def test_passes_when_nothing_is_reported(self, tmp_path):
        td.audit_check(
            _ctx(_CLEAN_REPORT), exceptions_file=_exceptions(tmp_path)
        )

    def test_unexcepted_high_advisory_fails_naming_it(self, tmp_path):
        with pytest.raises(Exit, match=_ADVISORY):
            td.audit_check(
                _ctx(_HIGH_REPORT), exceptions_file=_exceptions(tmp_path)
            )

    def test_unexcepted_critical_advisory_fails(self, tmp_path):
        report = _report(
            ("pkg", "critical", [_advisory(_ADVISORY, "critical")])
        )
        with pytest.raises(Exit, match=_ADVISORY):
            td.audit_check(_ctx(report), exceptions_file=_exceptions(tmp_path))

    def test_moderate_advisory_passes(self, tmp_path):
        report = _report(
            ("pkg", "moderate", [_advisory(_ADVISORY, "moderate")])
        )
        td.audit_check(_ctx(report), exceptions_file=_exceptions(tmp_path))

    def test_excepted_advisory_passes_along_with_its_dependants(self, tmp_path):
        td.audit_check(
            _ctx(_HIGH_REPORT),
            exceptions_file=_exceptions(tmp_path, _ADVISORY),
        )

    def test_an_exception_does_not_cover_other_advisories(self, tmp_path):
        other = "GHSA-aaaa-bbbb-cccc"
        report = _report(
            ("http-cache-semantics", "high", [_advisory(_ADVISORY, "high")]),
            ("devalue", "high", [_advisory(other, "high")]),
        )
        with pytest.raises(Exit, match=other):
            td.audit_check(
                _ctx(report), exceptions_file=_exceptions(tmp_path, _ADVISORY)
            )

    def test_failure_names_the_fix_command(self, tmp_path):
        with pytest.raises(Exit, match=r"mise run docs:audit:fix"):
            td.audit_check(
                _ctx(_HIGH_REPORT), exceptions_file=_exceptions(tmp_path)
            )

    def test_an_unreadable_report_fails(self, tmp_path):
        with pytest.raises(Exit, match="npm audit"):
            td.audit_check(
                _ctx('{"error": {"code": "ENOAUDIT"}}'),
                exceptions_file=_exceptions(tmp_path),
            )

    def test_the_shipped_exceptions_file_parses(self):
        td.audit_check(_ctx(_CLEAN_REPORT))
