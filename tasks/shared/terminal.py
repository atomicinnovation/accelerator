import re

_ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*m")


def without_ansi_escapes(text: str) -> str:
    return _ANSI_ESCAPE.sub("", text)
