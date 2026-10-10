"""The one HTTP client in the vendored-runtime pipeline.

Thin wrappers over ``httpx``: a streamed file download, JSON GET and POST, and
a size-capped byte GET. They are injected at the orchestration layer, so the
verification and guard logic is exercised against recorded fixtures rather
than the live registries, feeds and CDN.

Redirects are followed explicitly: httpx does not follow them by default (unlike
the registry and CDN endpoints, which redirect to their backing stores).

``httpx`` timeouts bound each socket read, so a body trickling in slowly could
outlast them indefinitely; the JSON and byte reads also enforce ``timeout`` as
a wall-clock total, checked as each chunk arrives. A read blocked waiting for
the next chunk can still overrun that total by one read timeout, because
``httpcore`` fixes the read timeout when the body starts streaming.
"""

import contextlib
import json
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

import httpx

type Fetcher = Callable[[str, Path], None]
type JsonFetcher = Callable[[str], dict[str, Any]]
type Now = Callable[[], float]

_CHUNK = 64 * 1024
_DOWNLOAD_TIMEOUT = 300
_JSON_TIMEOUT = 60


class PayloadTooLargeError(ValueError):
    def __init__(self, url: str, max_bytes: int) -> None:
        super().__init__(f"{url}: body exceeds {max_bytes} bytes")
        self.max_bytes = max_bytes


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


def get_json(
    url: str,
    *,
    timeout: float = _JSON_TIMEOUT,
    client: httpx.Client | None = None,
    now: Now = time.monotonic,
) -> dict[str, Any]:
    """GET ``url`` and return the parsed JSON body, raising on non-2xx."""
    return json.loads(
        _body("GET", url, timeout=timeout, client=client, now=now)
    )


def post_json(
    url: str,
    body: object,
    *,
    timeout: float,
    client: httpx.Client | None = None,
    now: Now = time.monotonic,
) -> object:
    return json.loads(
        _body(
            "POST", url, json_body=body, timeout=timeout, client=client, now=now
        )
    )


def get_bytes(
    url: str,
    *,
    timeout: float,
    max_bytes: int,
    client: httpx.Client | None = None,
    now: Now = time.monotonic,
) -> bytes:
    """GET ``url``'s body, refusing one longer than ``max_bytes``."""
    return _body(
        "GET",
        url,
        timeout=timeout,
        max_bytes=max_bytes,
        client=client,
        now=now,
    )


def _body(
    method: str,
    url: str,
    *,
    timeout: float,
    client: httpx.Client | None,
    now: Now,
    json_body: object = None,
    max_bytes: int | None = None,
) -> bytes:
    deadline = now() + timeout
    with contextlib.ExitStack() as stack:
        http = client or stack.enter_context(httpx.Client())
        response = stack.enter_context(
            http.stream(
                method,
                url,
                json=json_body,
                timeout=timeout,
                follow_redirects=True,
            )
        )
        response.raise_for_status()
        received = bytearray()
        for chunk in response.iter_bytes():
            received += chunk
            if max_bytes is not None and len(received) > max_bytes:
                raise PayloadTooLargeError(url, max_bytes)
            if now() >= deadline:
                raise httpx.ReadTimeout(
                    f"{url}: body took over {timeout}s",
                    request=response.request,
                )
        return bytes(received)
