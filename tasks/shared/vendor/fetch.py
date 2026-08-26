"""The one HTTP client in the vendored-runtime pipeline.

Thin wrappers over ``requests``: a streamed file download and a JSON GET. Both
are injected at the orchestration layer (`Fetcher`/`JsonFetcher`), so the
verification logic is exercised against recorded fixtures rather than the live
registry, nodejs.org and CDN.
"""

from collections.abc import Callable
from pathlib import Path
from typing import Any

import requests

type Fetcher = Callable[[str, Path], None]
type JsonFetcher = Callable[[str], dict[str, Any]]

_CHUNK = 64 * 1024
_DOWNLOAD_TIMEOUT = 300
_JSON_TIMEOUT = 60


def download(url: str, dest: Path) -> None:
    """Stream ``url`` to ``dest``, raising on any non-2xx response."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    with requests.get(url, stream=True, timeout=_DOWNLOAD_TIMEOUT) as response:
        response.raise_for_status()
        with dest.open("wb") as handle:
            for chunk in response.iter_content(chunk_size=_CHUNK):
                handle.write(chunk)


def get_json(url: str) -> dict[str, Any]:
    """GET ``url`` and return the parsed JSON body, raising on non-2xx."""
    response = requests.get(url, timeout=_JSON_TIMEOUT)
    response.raise_for_status()
    return response.json()
