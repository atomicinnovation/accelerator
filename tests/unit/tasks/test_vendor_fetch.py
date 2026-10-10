import json

import httpx
import pytest

from tasks.shared.vendor.fetch import (
    PayloadTooLargeError,
    get_bytes,
    get_json,
    post_json,
)
from tests.unit.tasks.shared.doubles import FakeClock


class TricklingBody(httpx.SyncByteStream):
    """A body whose every chunk takes ``seconds_per_chunk`` to arrive."""

    def __init__(self, chunks, clock, seconds_per_chunk):
        self.chunks = chunks
        self.clock = clock
        self.seconds_per_chunk = seconds_per_chunk

    def __iter__(self):
        for chunk in self.chunks:
            self.clock.t += self.seconds_per_chunk
            yield chunk


def _client(handler):
    return httpx.Client(transport=httpx.MockTransport(handler))


def test_post_json_sends_the_body_and_parses_the_reply():
    sent = []

    def handler(request):
        sent.append(json.loads(request.content))
        return httpx.Response(200, json={"results": []})

    reply = post_json(
        "https://feed.test/q",
        {"queries": [1]},
        timeout=20,
        client=_client(handler),
    )
    assert sent == [{"queries": [1]}]
    assert reply == {"results": []}


def test_get_json_parses_the_reply():
    client = _client(lambda _request: httpx.Response(200, json={"a": 1}))
    assert get_json("https://feed.test/a", timeout=20, client=client) == {
        "a": 1
    }


def test_a_non_2xx_status_raises():
    client = _client(lambda _request: httpx.Response(503))
    with pytest.raises(httpx.HTTPStatusError):
        get_json("https://feed.test/a", timeout=20, client=client)


def test_get_bytes_follows_a_redirect():
    def handler(request):
        if request.url.path == "/moved":
            return httpx.Response(
                302, headers={"location": "https://feed.test/here"}
            )
        return httpx.Response(200, content=b"payload")

    payload = get_bytes(
        "https://feed.test/moved",
        timeout=20,
        max_bytes=100,
        client=_client(handler),
    )
    assert payload == b"payload"


def test_get_bytes_refuses_a_body_past_its_cap():
    client = _client(lambda _request: httpx.Response(200, content=b"x" * 11))
    with pytest.raises(PayloadTooLargeError) as raised:
        get_bytes(
            "https://feed.test/big", timeout=20, max_bytes=10, client=client
        )
    assert raised.value.max_bytes == 10


def test_get_bytes_accepts_a_body_at_its_cap():
    client = _client(lambda _request: httpx.Response(200, content=b"x" * 10))
    assert (
        len(
            get_bytes(
                "https://feed.test/ok", timeout=20, max_bytes=10, client=client
            )
        )
        == 10
    )


def _trickling(clock, seconds_per_chunk):
    return _client(
        lambda _request: httpx.Response(
            200,
            stream=TricklingBody(
                [b'{"a"', b": ", b"1}"], clock, seconds_per_chunk
            ),
        )
    )


def test_a_body_trickling_past_the_total_timeout_raises():
    clock = FakeClock()
    with pytest.raises(httpx.TimeoutException):
        get_json(
            "https://feed.test/slow",
            timeout=20,
            client=_trickling(clock, 20 / 3),
            now=clock.now,
        )


def test_a_body_finishing_just_inside_the_total_timeout_returns():
    clock = FakeClock()
    reply = get_json(
        "https://feed.test/slow",
        timeout=20,
        client=_trickling(clock, 6.6),
        now=clock.now,
    )
    assert reply == {"a": 1}
