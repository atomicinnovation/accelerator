"""Unit tests for the environment the visualiser Playwright lane runs under."""

from tasks.test.e2e import SERVER_BIN, playwright_environment


def test_the_health_server_listens_on_a_freshly_allocated_port():
    env = playwright_environment(free_port=lambda: 54321)

    assert env["E2E_HEALTH_PORT"] == "54321"


def test_playwright_serves_the_dev_server_build():
    env = playwright_environment(free_port=lambda: 54321)

    assert env["ACCELERATOR_VISUALISER_BIN"] == str(SERVER_BIN)
