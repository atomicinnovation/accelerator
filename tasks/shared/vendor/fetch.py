"""The one HTTP client in the vendored-runtime pipeline.

Thin wrappers over ``httpx``: a streamed file download and a JSON GET. Both
are injected at the orchestration layer (`Fetcher`/`JsonFetcher`), so the
verification logic is exercised against recorded fixtures rather than the live
registry, nodejs.org and CDN.

Redirects are followed explicitly: httpx does not follow them by default (unlike
the registry and CDN endpoints, which redirect to their backing stores).
"""

from collections.abc import Callable
from pathlib import Path
from typing import Any

import httpx

type Fetcher = Callable[[str, Path], None]
type JsonFetcher = Callable[[str], dict[str, Any]]

_CHUNK = 64 * 1024
_DOWNLOAD_TIMEOUT = 300
_JSON_TIMEOUT = 60


def download(url: str, dest: Path) -> None:
    """Stream ``url`` to ``dest``, raising on any non-2xx response."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    with httpx.stream(
        "GET", url, timeout=_DOWNLOAD_TIMEOUT, follow_redirects=True
    ) as response:
        response.raise_for_status()
        with dest.open("wb") as handle:
            for chunk in response.iter_bytes(_CHUNK):
                handle.write(chunk)


def get_json(url: str) -> dict[str, Any]:
    """GET ``url`` and return the parsed JSON body, raising on non-2xx."""
    response = httpx.get(url, timeout=_JSON_TIMEOUT, follow_redirects=True)
    response.raise_for_status()
    return response.json()
