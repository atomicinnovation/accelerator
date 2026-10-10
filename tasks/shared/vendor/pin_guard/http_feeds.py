"""The HTTP adapter for the guard's feeds, with bounded retries."""

from collections.abc import Callable

import httpx

from tasks.shared.clock import Clock
from tasks.shared.vendor import fetch
from tasks.shared.vendor.pin_guard.feeds import (
    REQUEST_TIMEOUT_SECONDS,
    FeedDocumentError,
    FeedRecordNotFoundError,
    FeedUnreachableError,
)

RETRY_WAITS_SECONDS = (2, 8)
MAXIMUM_RETRY_AFTER_SECONDS = 30
_NOT_FOUND = 404
_TOO_MANY_REQUESTS = 429


class HttpFeedClient:
    """A ``FeedClient`` that reports failure only as feed-domain errors."""

    def __init__(
        self, clock: Clock, client: httpx.Client | None = None
    ) -> None:
        self._clock = clock
        self._client = client

    def get_json(self, url: str) -> object:
        return self._retrying(
            url,
            lambda: fetch.get_json(
                url,
                timeout=REQUEST_TIMEOUT_SECONDS,
                client=self._client,
                now=self._clock.now,
            ),
        )

    def post_json(self, url: str, body: object) -> object:
        return self._retrying(
            url,
            lambda: fetch.post_json(
                url,
                body,
                timeout=REQUEST_TIMEOUT_SECONDS,
                client=self._client,
                now=self._clock.now,
            ),
        )

    def get_bytes(self, url: str, max_bytes: int) -> bytes:
        return self._retrying(
            url,
            lambda: fetch.get_bytes(
                url,
                timeout=REQUEST_TIMEOUT_SECONDS,
                max_bytes=max_bytes,
                client=self._client,
                now=self._clock.now,
            ),
        )

    def _retrying[T](self, url: str, call: Callable[[], T]) -> T:
        for wait in (*RETRY_WAITS_SECONDS, None):
            try:
                return call()
            except fetch.PayloadTooLargeError as error:
                raise FeedDocumentError(
                    f"payload over {error.max_bytes} bytes"
                ) from error
            except ValueError as error:
                raise FeedDocumentError("unparseable") from error
            except httpx.HTTPStatusError as error:
                response = error.response
                if response.status_code == _NOT_FOUND:
                    raise FeedRecordNotFoundError(
                        f"{url}: HTTP {response.status_code}"
                    ) from error
                if wait is None or not _is_transient(response.status_code):
                    raise FeedUnreachableError(
                        f"{url}: HTTP {response.status_code}"
                    ) from error
                self._clock.sleep(_retry_after(response) or wait)
            except httpx.RequestError as error:
                if wait is None:
                    raise FeedUnreachableError(
                        f"{url}: {type(error).__name__} {error}"
                    ) from error
                self._clock.sleep(wait)
        raise AssertionError("the final attempt either returns or raises")


def _is_transient(status: int) -> bool:
    return status == _TOO_MANY_REQUESTS or status >= 500


def _retry_after(response: httpx.Response) -> int | None:
    value = response.headers.get("Retry-After", "")
    if not value.isdigit():
        return None
    return min(int(value), MAXIMUM_RETRY_AFTER_SECONDS)
