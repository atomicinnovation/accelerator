from invoke import Exit

from tasks.shared.selection import Selection


def selection_from(filter_: str | None) -> Selection:
    try:
        return Selection(filter_)
    except ValueError as error:
        raise Exit(str(error), code=1) from None
