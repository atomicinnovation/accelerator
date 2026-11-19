from tasks.shared.shared_fixtures.errors import SharedFixturesError


class TestSharedFixturesError:
    def test_names_each_problem_as_the_shared_fixtures(self):
        error = SharedFixturesError("it broke", "it leaked")

        assert str(error) == (
            "shared VCS fixtures: it broke\nshared VCS fixtures: it leaked"
        )

    def test_keeps_its_problems(self):
        assert SharedFixturesError("a", "b").problems == ("a", "b")

    def test_leads_with_the_failure_it_joins_as_it_was(self):
        error = SharedFixturesError(
            "it leaked", failure="nextest: cli tests failed"
        )

        assert str(error) == (
            "nextest: cli tests failed\nshared VCS fixtures: it leaked"
        )
