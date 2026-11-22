"""Read and rewrite a document's frontmatter fields, line by line."""

import json
import re
from typing import Any

_FENCE = "---"


def _block(document: str) -> list[str]:
    lines = document.splitlines()
    if not lines or lines[0] != _FENCE:
        return []
    rest = lines[1:]
    return rest[: rest.index(_FENCE)] if _FENCE in rest else rest


def read_fields(document: str) -> dict[str, Any]:
    fields: dict[str, Any] = {}
    for line in _block(document):
        key, separator, value = line.partition(": ")
        if separator:
            try:
                fields[key] = json.loads(value)
            except json.JSONDecodeError:
                fields[key] = value
    return fields


def with_fields(document: str, **fields: object) -> str:
    for key, value in fields.items():
        document, replaced = re.subn(
            rf"^{key}: .*$",
            f"{key}: {json.dumps(value, ensure_ascii=False)}",
            document,
            count=1,
            flags=re.MULTILINE,
        )
        if replaced != 1:
            raise ValueError(f"the seed document has no {key} field")
    return document
