import pytest

from tasks.shared.dev.health import Health, evaluate_health, status_exit_code


class TestEvaluateHealth:
    @pytest.mark.parametrize(
        "statuses, expected",
        [
            ({"server": "active", "frontend": "active"}, Health.HEALTHY),
            ({"server": "active", "frontend": "stopped"}, Health.PARTIAL),
            ({"server": "stopped", "frontend": "active"}, Health.PARTIAL),
            ({"server": "stopped", "frontend": "stopped"}, Health.DOWN),
            ({"server": "active"}, Health.PARTIAL),
            ({"frontend": "active"}, Health.PARTIAL),
            ({}, Health.DOWN),
            ({"server": "active", "frontend": "starting"}, Health.PARTIAL),
        ],
    )
    def test_maps_statuses_to_health(self, statuses, expected):
        assert evaluate_health(statuses) == expected


class TestStatusExitCode:
    @pytest.mark.parametrize(
        "health, code",
        [(Health.HEALTHY, 0), (Health.PARTIAL, 3), (Health.DOWN, 4)],
    )
    def test_maps_health_to_exit_code(self, health, code):
        assert status_exit_code(health) == code
