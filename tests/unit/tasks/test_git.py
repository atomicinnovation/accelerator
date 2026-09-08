from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.git as tg


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _commands(ctx):
    return [call.args[0] for call in ctx.run.call_args_list]


class TestPush:
    def test_pushes_to_origin_without_a_releaser_token(
        self, ctx, mocker, monkeypatch
    ):
        monkeypatch.delenv("RELEASER_TOKEN", raising=False)
        mocker.patch.object(tg.version, "read", return_value="1.2.3")

        tg.push(ctx)

        assert "git push --atomic origin HEAD 'refs/tags/v1.2.3'" in _commands(
            ctx
        )

    def test_pushes_over_a_token_authenticated_url_when_present(
        self, ctx, mocker, monkeypatch
    ):
        # The push credential must be minted at push time, not at checkout: a
        # GitHub App token expires after an hour and the cross-compile in
        # *:prepare routinely outlives it.
        monkeypatch.setenv("RELEASER_TOKEN", "ghs_secret_value")
        mocker.patch.object(tg.version, "read", return_value="1.2.3")

        def run(command, *args, **kwargs):
            if command == "git remote get-url origin":
                return MagicMock(
                    return_code=0,
                    stdout="https://github.com/atomicinnovation/accelerator\n",
                )
            return MagicMock(return_code=0, stdout="")

        ctx.run.side_effect = run

        tg.push(ctx)

        push = next(c for c in _commands(ctx) if c.startswith("git push"))
        assert (
            "x-access-token:${RELEASER_TOKEN}@github.com/"
            "atomicinnovation/accelerator" in push
        )
        # The shell expands the token at push time; Python must never bake the
        # literal secret into the command, or it would leak into process args.
        assert "ghs_secret_value" not in push
        assert "origin" not in push
