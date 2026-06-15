#!/usr/bin/env python3
import sys

if sys.version_info < (3, 9):
    sys.exit(
        "mock-linear-server.py requires Python 3.9+; "
        f"got {sys.version.split()[0]}"
    )

import argparse
import json
import signal
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse


def load_scenario(path: str) -> list[dict]:
    with open(path) as f:
        data = json.load(f)
    return data["expectations"]


class MockHandler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # suppress default access log

    def _fail(self, status: int, msg: str):
        """Answer with a non-2xx marker AND record a hard error so the
        scenario's captured-errors assertion fails. A flow that sends the
        wrong operation, in the wrong order, or omits a step cannot pass
        merely by positional consumption."""
        body = msg.encode()
        self.send_response(status)
        self.send_header("Content-Type", "text/plain")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        self.server.errors.append(msg)

    def do_request(self):
        server = self.server
        with server.lock:
            if not server.expectations:
                self.send_error(500, "No more expectations")
                server.errors.append("No more expectations")
                return

            exp = server.expectations[0]
            if exp.get("consume", True):
                server.expectations.pop(0)

        # Read the request body up front (so the connection stays clean and we
        # can match/capture it). Every Linear request is POST /graphql.
        content_length = int(self.headers.get("Content-Length", 0))
        request_body = b""
        if content_length:
            request_body = self.rfile.read(content_length)
        request_body_str = request_body.decode("utf-8", errors="replace")

        # Validate method and path (ignore query string in comparison)
        exp_method = exp.get("method", "POST")
        exp_path = exp.get("path", "/graphql")
        req_path = urlparse(self.path).path
        if self.command != exp_method or req_path != exp_path:
            self._fail(
                500,
                f"Unexpected request: {self.command} {req_path}; "
                f"expected {exp_method} {exp_path}",
            )
            return

        # Validate the request body contains the expected substring(s). A
        # mismatch is a hard failure (records an error AND answers non-2xx).
        expect_contains = exp.get("expect_body_contains")
        if expect_contains is not None:
            needles = (
                [expect_contains]
                if isinstance(expect_contains, str)
                else list(expect_contains)
            )
            missing = [n for n in needles if n not in request_body_str]
            if missing:
                self._fail(
                    422,
                    f"Body missing expected substrings {missing} on {req_path}",
                )
                return

        # Validate auth header if the expectation specifies it
        if "auth" in exp:
            actual_auth = self.headers.get("Authorization", "")
            expected_auth = exp["auth"]
            if actual_auth != expected_auth:
                self._fail(
                    401,
                    f"Auth mismatch: got '{actual_auth}', want '{expected_auth}'",
                )
                return

        # Validate custom headers
        for header, value in exp.get("expect_headers", {}).items():
            actual = self.headers.get(header, "")
            if actual != value:
                server.errors.append(
                    f"Header {header}: got '{actual}', want '{value}'"
                )

        # Optionally capture the full request URL (path + query string)
        if exp.get("capture_url", False):
            with server.lock:
                server.captured_urls.append(self.path)

        # Optionally capture the request body
        if exp.get("capture_body", False):
            with server.lock:
                server.captured_bodies.append(request_body_str)

        # Optionally capture the request headers (one dict per captured request)
        if exp.get("capture_headers", False):
            with server.lock:
                server.captured_headers.append(dict(self.headers.items()))

        resp = exp["response"]
        status = resp.get("status", 200)
        body_str = resp.get("body", "")
        body = body_str.encode() if isinstance(body_str, str) else body_str

        # Optional slow response (in seconds) for in-flight tests
        delay = resp.get("delay", 0)
        if delay:
            import time

            time.sleep(delay)

        self.send_response(status)
        for k, v in resp.get("headers", {}).items():
            self.send_header(k, v)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

        # Stop server after last expectation (when consume=true and list empty)
        with server.lock:
            if not server.expectations:
                threading.Thread(target=server.shutdown, daemon=True).start()

    do_GET = do_DELETE = do_POST = do_PUT = do_request


class MockServer(HTTPServer):
    def __init__(self, expectations: list[dict], port: int = 0):
        super().__init__(("127.0.0.1", port), MockHandler)
        self.expectations = list(expectations)
        self.errors: list[str] = []
        self.captured_bodies: list[str] = []
        self.captured_urls: list[str] = []
        self.captured_headers: list[dict] = []
        self.lock = threading.Lock()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--url-file", required=True)
    parser.add_argument("--captured-bodies-file", default="")
    parser.add_argument("--captured-urls-file", default="")
    parser.add_argument("--captured-headers-file", default="")
    parser.add_argument("--captured-errors-file", default="")
    parser.add_argument(
        "--port",
        type=int,
        default=0,
        help="Bind to a fixed port (default 0 = ephemeral). Used when a "
        "fixture must embed the mock URL ahead of launch (binary upload PUT).",
    )
    args = parser.parse_args()

    expectations = load_scenario(args.scenario)
    server = MockServer(expectations, port=args.port)
    port = server.server_address[1]
    url = f"http://127.0.0.1:{port}"

    with open(args.url_file, "w") as f:
        f.write(url)

    signal.signal(
        signal.SIGTERM,
        lambda *_: threading.Thread(
            target=server.shutdown, daemon=True
        ).start(),
    )

    try:
        server.serve_forever()
    finally:
        if args.captured_bodies_file:
            with open(args.captured_bodies_file, "w") as f:
                json.dump(server.captured_bodies, f)
        if args.captured_urls_file:
            with open(args.captured_urls_file, "w") as f:
                json.dump(server.captured_urls, f)
        if args.captured_headers_file:
            with open(args.captured_headers_file, "w") as f:
                json.dump(server.captured_headers, f)
        # The inherited stop_mock SIGTERMs the mock and discards its exit code,
        # so the shutdown sys.exit(1) is never observed. Instead, write the
        # captured errors to a file the suite reads back and asserts empty.
        if args.captured_errors_file:
            with open(args.captured_errors_file, "w") as f:
                json.dump(server.errors, f)
        if server.errors:
            print("MOCK ERRORS:", file=sys.stderr)
            for e in server.errors:
                print(f"  {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
