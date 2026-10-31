from tasks.shared.build_measurement.record_model import RecordModel


class CargoUnits(RecordModel):
    compiling: int = 0
    checking: int = 0
    documenting: int = 0
