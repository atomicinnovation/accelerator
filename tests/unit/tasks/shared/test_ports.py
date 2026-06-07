import socket

from tasks.shared.ports import free_port


class TestFreePort:
    def test_returns_a_bindable_loopback_port(self):
        port = free_port()
        assert isinstance(port, int)
        assert 1 <= port <= 65535
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", port))  # must be bindable right now
