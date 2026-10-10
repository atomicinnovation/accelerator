import httpx
import pytest

from tasks.shared.clock import Clock
from tasks.shared.vendor.pin_guard.feeds import (
    FeedDocumentError,
    FeedUnreachableError,
)
from tasks.shared.vendor.pin_guard.http_feeds import HttpFeedClient
from tests.unit.tasks.shared.doubles import FakeClock, osv_fixture

URL = "https://feed.test/a"


class Scripted:
    """A transport handler answering each request with the next response."""

    def __init__(self, *responses):
        self.responses = list(responses)
        self.requests = []

    def __call__(self, request):
        self.requests.append(request)
        response = self.responses.pop(0)
        if isinstance(response, Exception):
            raise response
        return response


class Trickling(httpx.SyncByteStream):
    def __init__(self, clock):
        self.clock = clock

    def __iter__(self):
        for chunk in (b'{"a"', b": 1}"):
            self.clock.t += 15
            yield chunk


def _client(handler, clock=None):
    clock = clock or FakeClock()
    feeds = HttpFeedClient(
        Clock(sleep=clock.sleep, now=clock.now),
        httpx.Client(transport=httpx.MockTransport(handler)),
    )
    return feeds, clock


def _ok():
    return httpx.Response(200, json={"a": 1})


def _refused():
    return httpx.ConnectError("refused")


def test_a_503_then_200_returns_the_body_after_one_wait():
    feeds, clock = _client(Scripted(httpx.Response(503), _ok()))
    assert feeds.get_json(URL) == {"a": 1}
    assert clock.sleeps == [2]


def test_a_transport_error_then_200_returns_the_body():
    feeds, clock = _client(Scripted(_refused(), _ok()))
    assert feeds.get_json(URL) == {"a": 1}
    assert clock.sleeps == [2]


def test_three_503s_are_unreachable_after_two_waits():
    feeds, clock = _client(Scripted(*[httpx.Response(503)] * 3))
    with pytest.raises(FeedUnreachableError, match="503"):
        feeds.get_json(URL)
    assert clock.sleeps == [2, 8]


def test_three_transport_errors_are_unreachable_after_two_waits():
    feeds, clock = _client(Scripted(*[_refused() for _ in range(3)]))
    with pytest.raises(FeedUnreachableError, match="ConnectError"):
        feeds.get_json(URL)
    assert clock.sleeps == [2, 8]


def test_a_body_overrunning_the_total_three_times_is_unreachable():
    clock = FakeClock()

    def trickle(_request):
        return httpx.Response(200, stream=Trickling(clock))

    feeds, _ = _client(trickle, clock)
    with pytest.raises(FeedUnreachableError):
        feeds.get_json(URL)
    assert clock.sleeps == [2, 8]


def test_a_429_then_200_waits_the_default():
    feeds, clock = _client(Scripted(httpx.Response(429), _ok()))
    assert feeds.get_json(URL) == {"a": 1}
    assert clock.sleeps == [2]


@pytest.mark.parametrize(("retry_after", "wait"), [("5", 5), ("120", 30)])
def test_a_429_honours_retry_after_up_to_a_cap(retry_after, wait):
    limited = httpx.Response(429, headers={"Retry-After": retry_after})
    feeds, clock = _client(Scripted(limited, _ok()))
    assert feeds.get_json(URL) == {"a": 1}
    assert clock.sleeps == [wait]


def test_a_404_is_unreachable_without_a_retry():
    not_found = httpx.Response(404, content=osv_fixture("not-found.json"))
    feeds, clock = _client(Scripted(not_found))
    with pytest.raises(FeedUnreachableError, match="404"):
        feeds.get_json(URL)
    assert clock.sleeps == []


def test_a_non_json_body_is_unparseable_without_a_retry():
    feeds, clock = _client(Scripted(httpx.Response(200, content=b"not json")))
    with pytest.raises(FeedDocumentError) as raised:
        feeds.get_json(URL)
    assert raised.value.reason == "unparseable"
    assert clock.sleeps == []


def test_post_json_sends_the_body():
    handler = Scripted(_ok())
    feeds, _ = _client(handler)
    assert feeds.post_json(URL, {"q": 1}) == {"a": 1}
    assert handler.requests[0].content == b'{"q":1}'


def test_get_bytes_returns_the_payload():
    feeds, _ = _client(Scripted(httpx.Response(200, content=b"abc")))
    assert feeds.get_bytes(URL, max_bytes=3) == b"abc"


def test_get_bytes_past_its_cap_is_a_document_error_without_a_retry():
    feeds, clock = _client(Scripted(httpx.Response(200, content=b"abcd")))
    with pytest.raises(FeedDocumentError) as raised:
        feeds.get_bytes(URL, max_bytes=3)
    assert raised.value.reason == "payload over 3 bytes"
    assert clock.sleeps == []
