"""Checks a scenario's JSON passes before it is read into typed values."""

from collections.abc import Mapping


def one_of[T: str](value: object, allowed: tuple[T, ...], what: str) -> T:
    for option in allowed:
        if value == option:
            return option
    raise ValueError(f"{value!r} is not a {what}")


def only(data: Mapping[str, object], keys: set[str], what: str) -> None:
    unknown = set(data) - keys
    if unknown:
        raise ValueError(f"a {what} cannot declare {sorted(unknown)}")
