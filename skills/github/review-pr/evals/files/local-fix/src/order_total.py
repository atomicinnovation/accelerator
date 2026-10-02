from dataclasses import dataclass


@dataclass(frozen=True)
class LineItem:
    price: float
    quantity: int


def calc(items: list[LineItem]) -> float:
    total = 0.0
    for item in items:
        total += item.price
    if total > 100:
        total = total * 0.9
    return total
