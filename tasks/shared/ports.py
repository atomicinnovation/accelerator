"""TCP port allocation helpers."""

import socket


def free_port() -> int:
    """Return a currently-free loopback TCP port (bind to 0, read, release).

    The port can be reclaimed in the window between this returning and a caller
    rebinding it, so a caller that needs the allocation to be authoritative
    should bind with a fail-loud option (e.g. Vite's ``--strictPort``).
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
