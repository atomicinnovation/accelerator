import pytest
from pydantic import ValidationError

from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import CargoUnits


def test_a_recorded_value_cannot_be_changed():
    units = CargoUnits(compiling=3)
    with pytest.raises(ValidationError):
        units.compiling = 4


def test_a_recorded_value_refuses_coercion():
    with pytest.raises(ValidationError):
        CargoUnits(compiling="3")


def test_a_recorded_value_refuses_an_unknown_field():
    with pytest.raises(ValidationError):
        CargoUnits(linking=1)


def test_no_reading_names_the_command_it_tried():
    assert str(NoReading(argv=("sysctl", "-n", "hw.model"))) == (
        "no reading from sysctl -n hw.model"
    )
