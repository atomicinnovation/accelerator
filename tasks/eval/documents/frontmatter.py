"""Rewrite a document's frontmatter fields in place, line by line."""

import json
import re


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
