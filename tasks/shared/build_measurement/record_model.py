import shlex
from typing import override

from pydantic import BaseModel, ConfigDict


class RecordModel(BaseModel):
    model_config = ConfigDict(frozen=True, strict=True, extra="forbid")


class NoReading(RecordModel):
    argv: tuple[str, ...]

    @override
    def __str__(self) -> str:
        return f"no reading from {shlex.join(self.argv)}"
