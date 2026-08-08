"""Unit tests for the file-descriptor limit raised ahead of zig links."""

import resource

import pytest

from tasks.shared.limits import (
    LINK_DESCRIPTOR_TARGET,
    descriptor_limit_raise_to,
    raise_descriptor_limit,
)

_INF = resource.RLIM_INFINITY


class TestDescriptorLimitRaiseTo:
    def test_raises_a_macos_default_soft_limit_to_the_target(self):
        # 256 is launchd's default and cannot cover a several-hundred-object
        # link; this is the case the guard exists for.
        assert descriptor_limit_raise_to(256, _INF) == LINK_DESCRIPTOR_TARGET

    def test_clamps_to_a_finite_hard_limit(self):
        # Linux reports a finite ceiling an unprivileged process cannot cross.
        assert descriptor_limit_raise_to(1024, 4096, wanted=65_536) == 4096

    @pytest.mark.parametrize(
        ("soft", "hard"),
        [
            (65_536, _INF),  # already exactly the target
            (1_048_576, _INF),  # already generous
            (_INF, _INF),  # already unlimited
        ],
    )
    def test_no_raise_when_the_limit_already_suffices(self, soft, hard):
        assert descriptor_limit_raise_to(soft, hard) is None

    def test_no_raise_when_the_hard_limit_is_at_or_below_the_soft_limit(self):
        # Clamping must never yield a lowering setrlimit call.
        assert descriptor_limit_raise_to(4096, 4096, wanted=65_536) is None
        assert descriptor_limit_raise_to(4096, 2048, wanted=65_536) is None


class TestRaiseDescriptorLimit:
    def test_raises_the_live_limit_and_restores_it(self):
        original = resource.getrlimit(resource.RLIMIT_NOFILE)
        soft, hard = original
        # Lower ourselves to the macOS default, raise back through the helper,
        # then restore — so the runner's own limit is unchanged either way.
        try:
            resource.setrlimit(resource.RLIMIT_NOFILE, (256, hard))
            assert raise_descriptor_limit(wanted=2048) == 2048
            assert resource.getrlimit(resource.RLIMIT_NOFILE)[0] == 2048
        finally:
            resource.setrlimit(resource.RLIMIT_NOFILE, original)
        assert resource.getrlimit(resource.RLIMIT_NOFILE) == original
        assert soft == original[0]

    def test_returns_none_without_touching_an_adequate_limit(self):
        before = resource.getrlimit(resource.RLIMIT_NOFILE)
        assert raise_descriptor_limit(wanted=1) is None
        assert resource.getrlimit(resource.RLIMIT_NOFILE) == before

    def test_a_refused_setrlimit_warns_and_continues(self, monkeypatch, capsys):
        def refuse(*_args, **_kwargs):
            raise OSError(1, "Operation not permitted")

        monkeypatch.setattr(resource, "getrlimit", lambda _which: (256, _INF))
        monkeypatch.setattr(resource, "setrlimit", refuse)
        assert raise_descriptor_limit() is None
        assert (
            "could not raise the file-descriptor limit"
            in capsys.readouterr().err
        )
