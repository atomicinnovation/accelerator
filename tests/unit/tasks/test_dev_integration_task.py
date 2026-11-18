from invoke import Context

from tasks.test.integration import dev


def test_the_dev_suite_spreads_its_tests_across_workers(mocker):
    context = mocker.MagicMock(spec=Context)

    dev(context)

    assert "-n auto" in context.run.call_args.args[0]
