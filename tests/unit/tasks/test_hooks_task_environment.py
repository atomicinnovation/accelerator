from invoke import Context

from tasks.test.integration import hooks


def test_the_hooks_suite_is_given_no_environment_overlay(mocker):
    context = mocker.MagicMock(spec=Context)

    hooks(context)

    assert "env" not in context.run.call_args.kwargs
