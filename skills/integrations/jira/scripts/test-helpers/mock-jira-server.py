#!/usr/bin/env python3
import sys

if sys.version_info < (3, 9):
    sys.exit(
        "mock-jira-server.py requires Python 3.9+; "
        f"got {sys.version.split()[0]}"
    )

import argparse
import json
import signal
import socket
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

    def do_request(self):
        server = self.server
        with server.lock:
            if not server.expectations:
                self.send_error(500, "No more expectations")
                return

            exp = server.expectations[0]
            if exp.get("consume", True):
                server.expectations.pop(0)

        # Validate method and path (ignore query string in comparison)
        exp_method = exp.get("method", "GET")
        exp_path = exp.get("path", "")
        req_path = urlparse(self.path).path
        if self.command != exp_method or req_path != exp_path:
            body = (
                f"Unexpected request: {self.command} {req_path}\n"
                f"Expected:           {exp_method} {exp_path}"
            ).encode()
            self.send_response(500)
            self.send_header("Content-Type", "text/plain")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            server.errors.append(f"Unexpected: {self.command} {req_path}")
            return

        # Validate auth header if expectation specifies it
        if "auth" in exp:
            actual_auth = self.headers.get("Authorization", "")
            expected_auth = exp["auth"]
            if actual_auth != expected_auth:
                body = f"Auth mismatch: got '{actual_auth}', want '{expected_auth}'".encode()
                self.send_response(401)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
                server.errors.append(f"Auth mismatch on {self.path}")
                return

        # Validate custom headers
        for header, value in exp.get("expect_headers", {}).items():
            actual = self.headers.get(header, "")
            if actual != value:
                server.errors.append(
                    f"Header {header}: got '{actual}', want '{value}'"
                )

        # Consume request body (so connection stays clean)
        content_length = int(self.headers.get("Content-Length", 0))
        if content_length:
            self.rfile.read(content_length)

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

        # Stop server after last expectation (when consume=true and list now empty)
        with server.lock:
            if not server.expectations:
                threading.Thread(target=server.shutdown, daemon=True).start()

    do_GET = do_DELETE = do_POST = do_PUT = do_request


class MockServer(HTTPServer):
    def __init__(self, expectations: list[dict]):
        super().__init__(("127.0.0.1", 0), MockHandler)
        self.expectations = list(expectations)
        self.errors: list[str] = []
        self.lock = threading.Lock()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--url-file", required=True)
    args = parser.parse_args()

    expectations = load_scenario(args.scenario)
    server = MockServer(expectations)
    port = server.server_address[1]
    url = f"http://127.0.0.1:{port}"

    with open(args.url_file, "w") as f:
        f.write(url)

    signal.signal(signal.SIGTERM, lambda *_: threading.Thread(target=server.shutdown, daemon=True).start())

    try:
        server.serve_forever()
    finally:
        if server.errors:
            print("MOCK ERRORS:", file=sys.stderr)
            for e in server.errors:
                print(f"  {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
