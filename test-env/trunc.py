"""Truncating HTTP endpoint for RECORD-CASES §7-1: 200 + full Content-Length,
then disconnect after N bytes without completing the response body.

Serves only 127.0.0.1, only under /trunc/<file>/<bytes>. Caddy proxies /trunc/*
to this server; normal file traffic stays in Caddy's file_server.
"""

import os
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.abspath(__file__))
FIXTURES = os.path.join(ROOT, "fixtures", "http")
PORT = int(os.environ.get("TRUNC_PORT", "18081"))


class TruncHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        parts = self.path.strip("/").split("/")
        if len(parts) != 3 or parts[0] != "trunc" or not parts[2].isdigit():
            self.send_error(404, "expected /trunc/<file>/<bytes>")
            return
        name, count = parts[1], int(parts[2])
        # Reject traversal: only a plain file name directly under fixtures/http.
        if os.path.basename(name) != name or ".." in name:
            self.send_error(404, "invalid file")
            return
        path = os.path.join(FIXTURES, name)
        if not os.path.isfile(path):
            self.send_error(404, "no such fixture")
            return
        size = os.path.getsize(path)
        self.send_response(200)
        self.send_header("Content-Length", str(size))
        self.send_header("Content-Type", "application/octet-stream")
        self.end_headers()
        with open(path, "rb") as handle:
            self.wfile.write(handle.read(min(count, size)))
        self.wfile.flush()
        # Deliberate protocol violation: close instead of completing the body.
        self.close_connection = True

    def log_message(self, fmt, *args):
        pass


if __name__ == "__main__":
    server = ThreadingHTTPServer(("127.0.0.1", PORT), TruncHandler)
    print(f"trunc endpoint listening on 127.0.0.1:{PORT}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.exit(0)
